use std::collections::BTreeMap;
use std::num::NonZeroU16;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::AtomicU32;
use std::sync::atomic::Ordering;
use std::thread::Thread;
use std::time::Duration;
use std::time::Instant;

use zeta_remote::SshHost;
use zeta_remote_connections::SshTunnel;
use zeta_remote_connections::SshTunnelOptions;
use zeta_remote_connections::select_available_loopback_port;
use zeta_winit::EventLoopProxy;

use crate::native_event::NativeEvent;
use crate::remote_tunnel_readiness::RemoteTunnelStartup;
use crate::remote_tunnel_readiness::wait_for_remote_tunnel;

const PROCESS_POLL_INTERVAL: Duration = Duration::from_millis(100);
const RECOVERY_WINDOW: Duration = Duration::from_secs(30);
const INITIAL_RECOVERY_DELAY: Duration = Duration::from_millis(250);
const MAX_RECOVERY_DELAY: Duration = Duration::from_secs(2);
static NEXT_TUNNEL_ID: AtomicU32 = AtomicU32::new(1);

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum RemoteTunnelUpdate {
    Ready { local_port: NonZeroU16 },
    Recovering { attempt: u32 },
    Stopped,
    Failed(String),
}

impl RemoteTunnelUpdate {
    pub(crate) const fn is_terminal(&self) -> bool {
        matches!(self, Self::Stopped | Self::Failed(_))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct RemoteTunnelEvent {
    tunnel_id: u32,
    remote_port: NonZeroU16,
    update: RemoteTunnelUpdate,
}

impl RemoteTunnelEvent {
    #[cfg(test)]
    pub(crate) const fn new(
        tunnel_id: u32,
        remote_port: NonZeroU16,
        update: RemoteTunnelUpdate,
    ) -> Self {
        Self {
            tunnel_id,
            remote_port,
            update,
        }
    }

    pub(crate) const fn tunnel_id(&self) -> u32 {
        self.tunnel_id
    }

    pub(crate) const fn remote_port(&self) -> NonZeroU16 {
        self.remote_port
    }

    pub(crate) const fn update(&self) -> &RemoteTunnelUpdate {
        &self.update
    }
}

#[derive(Clone, Debug)]
struct RemoteTunnelTarget {
    host: SshHost,
    ssh_executable: PathBuf,
}

/// Native-host owner for every SSH tunnel attached to one Remote zeterm window.
///
/// The renderer never receives credentials or a child-process handle. Dropping this owner signals
/// every worker to terminate its OpenSSH child, so a tunnel cannot outlive the product window.
pub(crate) struct RemoteTunnelHost {
    target: RemoteTunnelTarget,
    processes: BTreeMap<u32, RemoteTunnelProcess>,
}

impl RemoteTunnelHost {
    pub(crate) fn new(host: SshHost, ssh_executable: impl Into<PathBuf>) -> Self {
        Self {
            target: RemoteTunnelTarget {
                host,
                ssh_executable: ssh_executable.into(),
            },
            processes: BTreeMap::new(),
        }
    }

    pub(crate) fn host(&self) -> &SshHost {
        &self.target.host
    }

    pub(crate) fn start(
        &mut self,
        remote_port: NonZeroU16,
        event_proxy: EventLoopProxy<NativeEvent>,
    ) -> Result<u32, String> {
        let process = spawn_remote_tunnel(self.target.clone(), remote_port, move |event| {
            let _ = event_proxy.send_event(NativeEvent::RemoteTunnel(event));
        })?;
        let tunnel_id = process.tunnel_id();
        self.processes.insert(tunnel_id, process);
        Ok(tunnel_id)
    }

    pub(crate) fn stop(&self, tunnel_id: u32) -> bool {
        self.processes.get(&tunnel_id).is_some_and(|process| {
            process.cancel();
            true
        })
    }

    pub(crate) fn handle_event(&mut self, event: &RemoteTunnelEvent) -> bool {
        if !self.processes.contains_key(&event.tunnel_id) {
            return false;
        }
        if event.update.is_terminal() {
            self.processes.remove(&event.tunnel_id);
        }
        true
    }
}

struct RemoteTunnelProcess {
    tunnel_id: u32,
    cancelled: Arc<AtomicBool>,
    worker_thread: Thread,
    worker: Option<std::thread::JoinHandle<()>>,
}

impl RemoteTunnelProcess {
    const fn tunnel_id(&self) -> u32 {
        self.tunnel_id
    }

    fn cancel(&self) {
        self.cancelled.store(true, Ordering::Release);
        self.worker_thread.unpark();
    }
}

impl Drop for RemoteTunnelProcess {
    fn drop(&mut self) {
        self.cancel();
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

fn spawn_remote_tunnel(
    target: RemoteTunnelTarget,
    remote_port: NonZeroU16,
    send: impl Fn(RemoteTunnelEvent) + Send + 'static,
) -> Result<RemoteTunnelProcess, String> {
    let tunnel_id = NEXT_TUNNEL_ID
        .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |current| {
            current.checked_add(1)
        })
        .map_err(|_| "SSH tunnel identity space is exhausted".to_owned())?;
    let cancelled = Arc::new(AtomicBool::new(false));
    let worker_cancelled = Arc::clone(&cancelled);
    let worker = std::thread::Builder::new()
        .name(format!("zeterm-remote-tunnel-{tunnel_id}"))
        .spawn(move || run_remote_tunnel(tunnel_id, target, remote_port, &worker_cancelled, &send))
        .map_err(|error| format!("could not start SSH tunnel supervisor: {error}"))?;
    let worker_thread = worker.thread().clone();
    Ok(RemoteTunnelProcess {
        tunnel_id,
        cancelled,
        worker_thread,
        worker: Some(worker),
    })
}

fn run_remote_tunnel(
    tunnel_id: u32,
    target: RemoteTunnelTarget,
    remote_port: NonZeroU16,
    cancelled: &AtomicBool,
    send: &impl Fn(RemoteTunnelEvent),
) {
    let result = run_remote_tunnel_inner(tunnel_id, target, remote_port, cancelled, send);
    if let Err(error) = result {
        send(RemoteTunnelEvent {
            tunnel_id,
            remote_port,
            update: RemoteTunnelUpdate::Failed(error),
        });
    }
}

fn run_remote_tunnel_inner(
    tunnel_id: u32,
    target: RemoteTunnelTarget,
    remote_port: NonZeroU16,
    cancelled: &AtomicBool,
    send: &impl Fn(RemoteTunnelEvent),
) -> Result<(), String> {
    let local_port = select_available_loopback_port().map_err(|error| error.to_string())?;
    let mut tunnel = match start_tunnel(&target, local_port, remote_port, cancelled)? {
        RemoteTunnelStartup::Ready(tunnel) => tunnel,
        RemoteTunnelStartup::Cancelled => {
            send_stopped(send, tunnel_id, remote_port);
            return Ok(());
        }
    };
    if cancelled.load(Ordering::Acquire) {
        tunnel.stop().map_err(|error| error.to_string())?;
        send_stopped(send, tunnel_id, remote_port);
        return Ok(());
    }
    send(RemoteTunnelEvent {
        tunnel_id,
        remote_port,
        update: RemoteTunnelUpdate::Ready { local_port },
    });
    loop {
        if cancelled.load(Ordering::Acquire) {
            tunnel.stop().map_err(|error| error.to_string())?;
            send_stopped(send, tunnel_id, remote_port);
            return Ok(());
        }
        let failure = match tunnel.try_wait() {
            Ok(Some(status)) => format!("SSH tunnel exited: {status}"),
            Err(error) => format!("could not inspect SSH tunnel: {error}"),
            Ok(None) => {
                wait_unless_cancelled(cancelled, PROCESS_POLL_INTERVAL);
                continue;
            }
        };
        drop(tunnel);
        let Some(recovered) = recover_tunnel(
            send,
            tunnel_id,
            &target,
            local_port,
            remote_port,
            cancelled,
            failure,
        )?
        else {
            return Ok(());
        };
        tunnel = recovered;
    }
}

fn start_tunnel(
    target: &RemoteTunnelTarget,
    local_port: NonZeroU16,
    remote_port: NonZeroU16,
    cancelled: &AtomicBool,
) -> Result<RemoteTunnelStartup, String> {
    let tunnel = SshTunnelOptions::new(target.host.clone(), local_port, remote_port)
        .with_ssh_executable(target.ssh_executable.clone())
        .start()
        .map_err(|error| error.to_string())?;
    wait_for_remote_tunnel(tunnel, || cancelled.load(Ordering::Acquire))
}

fn recover_tunnel(
    send: &impl Fn(RemoteTunnelEvent),
    tunnel_id: u32,
    target: &RemoteTunnelTarget,
    local_port: NonZeroU16,
    remote_port: NonZeroU16,
    cancelled: &AtomicBool,
    mut last_failure: String,
) -> Result<Option<SshTunnel>, String> {
    let started = Instant::now();
    let mut attempts = 0_usize;
    loop {
        if cancelled.load(Ordering::Acquire) {
            send_stopped(send, tunnel_id, remote_port);
            return Ok(None);
        }
        let Some(delay) = recovery_delay_within_window(started.elapsed(), attempts) else {
            return Err(format!(
                "SSH tunnel did not recover within {} seconds after {attempts} attempts: {last_failure}",
                RECOVERY_WINDOW.as_secs()
            ));
        };
        attempts += 1;
        send(RemoteTunnelEvent {
            tunnel_id,
            remote_port,
            update: RemoteTunnelUpdate::Recovering {
                attempt: attempts as u32,
            },
        });
        if !wait_unless_cancelled(cancelled, delay) {
            send_stopped(send, tunnel_id, remote_port);
            return Ok(None);
        }
        match start_tunnel(target, local_port, remote_port, cancelled) {
            Ok(RemoteTunnelStartup::Ready(tunnel)) => {
                if cancelled.load(Ordering::Acquire) {
                    tunnel.stop().map_err(|error| error.to_string())?;
                    send_stopped(send, tunnel_id, remote_port);
                    return Ok(None);
                }
                send(RemoteTunnelEvent {
                    tunnel_id,
                    remote_port,
                    update: RemoteTunnelUpdate::Ready { local_port },
                });
                return Ok(Some(tunnel));
            }
            Ok(RemoteTunnelStartup::Cancelled) => {
                send_stopped(send, tunnel_id, remote_port);
                return Ok(None);
            }
            Err(error) => last_failure = error,
        }
    }
}

fn recovery_delay_within_window(elapsed: Duration, attempt: usize) -> Option<Duration> {
    let remaining = RECOVERY_WINDOW.checked_sub(elapsed)?;
    let delay = recovery_delay(attempt);
    (delay <= remaining).then_some(delay)
}

fn recovery_delay(attempt: usize) -> Duration {
    let multiplier = 1_u32 << (attempt.min(31) as u32);
    (INITIAL_RECOVERY_DELAY * multiplier).min(MAX_RECOVERY_DELAY)
}

fn wait_unless_cancelled(cancelled: &AtomicBool, delay: Duration) -> bool {
    let started = Instant::now();
    loop {
        if cancelled.load(Ordering::Acquire) {
            return false;
        }
        let Some(remaining) = delay.checked_sub(started.elapsed()) else {
            return true;
        };
        std::thread::park_timeout(remaining);
    }
}

fn send_stopped(send: &impl Fn(RemoteTunnelEvent), tunnel_id: u32, remote_port: NonZeroU16) {
    send(RemoteTunnelEvent {
        tunnel_id,
        remote_port,
        update: RemoteTunnelUpdate::Stopped,
    });
}

#[cfg(test)]
#[path = "remote_tunnel_process_tests.rs"]
mod tests;
