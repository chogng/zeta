use std::collections::HashMap;
use std::sync::Arc;

use anyhow::{Context, Result};
use tokio::runtime::Runtime;
use zeta_terminal::{GridSize, TerminalCore};
use zeta_terminal_workspace::TerminalReady;
pub(crate) use zeta_terminal_workspace::TerminalSessionKey;
use zeta_utils_pty::{ProcessHandle, SpawnedProcess, TerminalSize, spawn_pty_process};
use zui::app::AppProxy;

use crate::PRODUCT_DISPLAY_NAME;
use crate::app_server::AppServerHost;
use crate::native_event::NativeEvent;
use crate::session::session_switch_trace;

#[path = "terminal_session/remote.rs"]
mod remote;
use remote::RemoteTerminalBackend;

const SHELL_BOOTSTRAP_MARKER: &[u8] = b"\x1b]9;app-ready\x07";

#[derive(Debug)]
pub(crate) enum TerminalSessionEvent {
    Output(Vec<u8>),
    Exited(i32),
}

pub(crate) struct TerminalSessionEventEnvelope {
    pub(crate) key: TerminalSessionKey,
    pub(crate) event: TerminalSessionEvent,
}

impl TerminalSessionEventEnvelope {
    pub(crate) const fn new(key: TerminalSessionKey, event: TerminalSessionEvent) -> Self {
        Self { key, event }
    }
}

pub(crate) type TerminalSessionReady = TerminalReady<TerminalSession>;

impl TerminalSessionEvent {
    fn apply_to(self, core: &mut TerminalCore) {
        match self {
            Self::Output(bytes) => core.process_output(&bytes),
            Self::Exited(exit_code) => core.mark_process_exited(exit_code),
        }
    }
}

pub(crate) struct TerminalSession {
    backend: TerminalBackend,
    core: TerminalCore,
    size: GridSize,
}

impl Drop for TerminalSession {
    fn drop(&mut self) {
        match &mut self.backend {
            TerminalBackend::Local { process, .. } => process.request_terminate(),
            TerminalBackend::Remote(_) => {}
        }
    }
}

enum TerminalBackend {
    Local {
        _runtime: Runtime,
        process: Arc<ProcessHandle>,
    },
    Remote(RemoteTerminalBackend),
}

impl TerminalSession {
    /// Creates one terminal runtime on a short-lived worker and posts its result to the native
    /// event loop. The worker owns the potentially blocking PTY setup; the event loop only adopts
    /// the finished `TerminalSession` when it receives `TerminalSessionReady`.
    pub(crate) fn spawn_async(
        key: TerminalSessionKey,
        size: GridSize,
        event_proxy: AppProxy<NativeEvent>,
        target: AppServerHost,
    ) -> Result<()> {
        session_switch_trace::event(
            None,
            "terminal-spawn-queued",
            format_args!("key={key:?} rows={} cols={}", size.rows(), size.cols()),
        );
        std::thread::Builder::new()
            .name("app-terminal-spawn".to_owned())
            .spawn(move || {
                session_switch_trace::event(
                    None,
                    "terminal-spawn-start",
                    format_args!("key={key:?}"),
                );
                let result = Self::spawn(key, size, event_proxy.clone(), target)
                    .map_err(|error| format!("{error:#}"));
                session_switch_trace::event(
                    None,
                    "terminal-spawn-finished",
                    format_args!("key={key:?} success={}", result.is_ok()),
                );
                let _ = event_proxy.send_event(TerminalReady::new(key, result).into());
            })
            .context("could not start terminal spawn worker")?;
        Ok(())
    }

    pub(crate) fn spawn(
        key: TerminalSessionKey,
        size: GridSize,
        event_proxy: AppProxy<NativeEvent>,
        target: AppServerHost,
    ) -> Result<Self> {
        let _trace = session_switch_trace::Span::new(None, "terminal-session-spawn");
        let Some(connection) = target.remote_connection() else {
            return Self::spawn_local(key, size, event_proxy);
        };
        Ok(Self {
            backend: TerminalBackend::Remote(RemoteTerminalBackend::spawn(
                key,
                size,
                event_proxy,
                connection.clone(),
            )?),
            core: TerminalCore::new(size),
            size,
        })
    }

    fn spawn_local(
        key: TerminalSessionKey,
        size: GridSize,
        event_proxy: AppProxy<NativeEvent>,
    ) -> Result<Self> {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .thread_name("app-terminal")
            .build()
            .context("could not create terminal runtime")?;
        let program = default_shell();
        let cwd =
            std::env::current_dir().context("could not resolve terminal working directory")?;
        let environment = terminal_environment();
        let spawned = runtime
            .block_on(spawn_pty_process(
                &program,
                &[],
                &cwd,
                &environment,
                &None,
                TerminalSize {
                    rows: size.rows(),
                    cols: size.cols(),
                },
                &[],
            ))
            .with_context(|| format!("could not start shell {program}"))?;
        let bootstrap = shell_bootstrap(&program);
        let process = spawn_event_forwarders(
            &runtime,
            spawned,
            key,
            event_proxy,
            bootstrap.as_ref().map(|bootstrap| bootstrap.marker),
        );
        if let Some(bootstrap) = bootstrap {
            process
                .writer_sender()
                .try_send(bootstrap.input)
                .context("terminal bootstrap queue is unavailable")?;
        }
        Ok(Self {
            backend: TerminalBackend::Local {
                _runtime: runtime,
                process,
            },
            core: TerminalCore::new(size),
            size,
        })
    }

    pub(crate) const fn core(&self) -> &TerminalCore {
        &self.core
    }

    pub(crate) fn handle_event(&mut self, event: TerminalSessionEvent) -> Result<()> {
        event.apply_to(&mut self.core);
        let replies = self.core.take_reply_bytes();
        self.send_input(replies)
    }

    pub(crate) fn send_input(&mut self, input: Vec<u8>) -> Result<()> {
        if input.is_empty() {
            return Ok(());
        }
        match &self.backend {
            TerminalBackend::Local { process, .. } => process
                .writer_sender()
                .try_send(input)
                .context("terminal input queue is unavailable"),
            TerminalBackend::Remote(remote) => remote.send_input(input),
        }
    }

    pub(crate) fn resize(&mut self, size: GridSize) -> Result<()> {
        if self.size == size {
            return Ok(());
        }
        self.core.resize(size);
        self.size = size;
        match &self.backend {
            TerminalBackend::Local { process, .. } => process
                .resize(TerminalSize {
                    rows: size.rows(),
                    cols: size.cols(),
                })
                .context("could not resize terminal PTY"),
            TerminalBackend::Remote(remote) => remote.resize(size),
        }
    }
}

fn spawn_event_forwarders(
    runtime: &Runtime,
    spawned: SpawnedProcess,
    key: TerminalSessionKey,
    event_proxy: AppProxy<NativeEvent>,
    suppress_until: Option<&'static [u8]>,
) -> Arc<ProcessHandle> {
    let SpawnedProcess {
        session,
        mut stdout_rx,
        stderr_rx: _,
        mut exit_rx,
    } = spawned;
    let process = Arc::new(session);
    let event_process = Arc::clone(&process);
    let mut bootstrap_filter = suppress_until.map(BootstrapOutputFilter::new);
    runtime.spawn(async move {
        let exit_code = loop {
            tokio::select! {
                output = stdout_rx.recv() => match output {
                    Some(bytes) => {
                        let output = filter_bootstrap_output(&mut bootstrap_filter, bytes);
                        let Some(output) = output else {
                            continue;
                        };
                        if event_proxy
                            .send_event(
                                TerminalSessionEventEnvelope::new(
                                    key,
                                    TerminalSessionEvent::Output(output),
                                )
                                .into(),
                            )
                            .is_err()
                        {
                            return;
                        }
                    }
                    None => break exit_rx.await.unwrap_or(-1),
                },
                exit = &mut exit_rx => {
                    let exit_code = exit.unwrap_or(-1);
                    event_process.release_pty_handles_after_exit();
                    while let Some(bytes) = stdout_rx.recv().await {
                        let Some(output) = filter_bootstrap_output(&mut bootstrap_filter, bytes)
                        else {
                            continue;
                        };
                        if event_proxy
                            .send_event(
                                TerminalSessionEventEnvelope::new(
                                    key,
                                    TerminalSessionEvent::Output(output),
                                )
                                .into(),
                            )
                            .is_err()
                        {
                            return;
                        }
                    }
                    break exit_code;
                }
            }
        };
        event_process.release_pty_handles_after_exit();
        let _ = event_proxy.send_event(
            TerminalSessionEventEnvelope::new(key, TerminalSessionEvent::Exited(exit_code)).into(),
        );
    });
    process
}

fn filter_bootstrap_output(
    filter: &mut Option<BootstrapOutputFilter>,
    output: Vec<u8>,
) -> Option<Vec<u8>> {
    let output = match filter.as_mut() {
        Some(filter) => filter.push(output),
        None => Some(output),
    }?;
    *filter = None;
    Some(output)
}

struct ShellBootstrap {
    input: Vec<u8>,
    marker: &'static [u8],
}

fn shell_bootstrap(program: &str) -> Option<ShellBootstrap> {
    let shell = std::path::Path::new(program).file_name()?.to_str()?;
    if !matches!(shell, "bash" | "dash" | "ksh" | "sh" | "zsh") {
        return None;
    }
    let input = if shell == "zsh" {
        b"unsetopt zle; stty -echo; function __app_precmd { local status=$?; printf '\\033]133;D;%d\\007' \"$status\"; }; precmd_functions=(__app_precmd); PROMPT=$'\\033[0m'; RPROMPT=''; PROMPT_EOL_MARK=''; printf '\\033]9;app-ready\\007'\r".to_vec()
    } else {
        b"stty -echo; PS1=''; PS2=''; printf '\\033]9;app-ready\\007'\r".to_vec()
    };
    Some(ShellBootstrap {
        input,
        marker: SHELL_BOOTSTRAP_MARKER,
    })
}

struct BootstrapOutputFilter {
    marker: &'static [u8],
    buffered: Vec<u8>,
}

impl BootstrapOutputFilter {
    fn new(marker: &'static [u8]) -> Self {
        Self {
            marker,
            buffered: Vec::new(),
        }
    }

    fn push(&mut self, output: Vec<u8>) -> Option<Vec<u8>> {
        self.buffered.extend(output);
        let marker_start = self
            .buffered
            .windows(self.marker.len())
            .position(|window| window == self.marker)?;
        let remainder_start = marker_start + self.marker.len();
        Some(self.buffered.split_off(remainder_start))
    }
}

fn terminal_environment() -> HashMap<String, String> {
    let mut environment = std::env::vars().collect::<HashMap<_, _>>();
    environment.insert("TERM".to_string(), "xterm-256color".to_string());
    environment.insert("COLORTERM".to_string(), "truecolor".to_string());
    environment.insert("TERM_PROGRAM".to_string(), PRODUCT_DISPLAY_NAME.to_string());
    environment
}

#[cfg(unix)]
fn default_shell() -> String {
    std::env::var("SHELL")
        .ok()
        .filter(|shell| !shell.is_empty())
        .unwrap_or_else(|| "/bin/sh".to_string())
}

#[cfg(windows)]
fn default_shell() -> String {
    std::env::var("COMSPEC")
        .ok()
        .filter(|shell| !shell.is_empty())
        .unwrap_or_else(|| "powershell.exe".to_string())
}

#[cfg(not(any(unix, windows)))]
fn default_shell() -> String {
    "sh".to_string()
}

#[cfg(test)]
#[path = "terminal_session/tests.rs"]
mod tests;
