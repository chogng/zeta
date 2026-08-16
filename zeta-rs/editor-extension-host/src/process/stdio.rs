use std::collections::BTreeMap;
use std::collections::VecDeque;
use std::io::BufRead;
use std::io::BufReader;
use std::io::BufWriter;
use std::io::Read;
use std::io::Write;
use std::process::Child;
use std::process::ChildStderr;
use std::process::ChildStdin;
use std::process::ChildStdout;
use std::process::Command;
use std::process::Stdio;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::thread;
use std::thread::JoinHandle;

use super::ExtensionHostProcess;
use super::ExtensionLaunchCommand;
use super::PendingEntry;
use super::PendingFailure;
use super::PendingHostRequest;
use super::reserve_pending;
use crate::ExtensionHostError;
use crate::ExtensionHostLimits;
use crate::ExtensionHostOutputEvent;
use crate::ExtensionHostRequest;
use crate::protocol::ExtensionHostStdoutFrame;

#[derive(Default)]
struct OutputEventQueue {
    events: VecDeque<ExtensionHostOutputEvent>,
    bytes: usize,
}

pub(super) struct StdioExtensionHostProcess {
    child: Mutex<Option<Child>>,
    writer: Mutex<Option<BufWriter<ChildStdin>>>,
    pending: Arc<Mutex<BTreeMap<u64, PendingEntry>>>,
    exited: Arc<AtomicBool>,
    stderr: Arc<Mutex<Vec<u8>>>,
    output_events: Arc<Mutex<OutputEventQueue>>,
    stdout_thread: Mutex<Option<JoinHandle<()>>>,
    stderr_thread: Mutex<Option<JoinHandle<()>>>,
    limits: ExtensionHostLimits,
}

impl StdioExtensionHostProcess {
    pub(super) fn spawn(
        launch: &ExtensionLaunchCommand,
        limits: &ExtensionHostLimits,
    ) -> Result<Self, ExtensionHostError> {
        let mut command = Command::new(launch.executable());
        command
            .args(launch.arguments())
            .current_dir(launch.working_directory())
            .env_clear()
            .envs(launch.environment())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let mut child = command
            .spawn()
            .map_err(|_| ExtensionHostError::SpawnFailed)?;
        let stdin = child.stdin.take().ok_or(ExtensionHostError::SpawnFailed)?;
        let stdout = child.stdout.take().ok_or(ExtensionHostError::SpawnFailed)?;
        let stderr_pipe = child.stderr.take().ok_or(ExtensionHostError::SpawnFailed)?;
        let pending = Arc::new(Mutex::new(BTreeMap::new()));
        let exited = Arc::new(AtomicBool::new(false));
        let stderr = Arc::new(Mutex::new(Vec::new()));
        let output_events = Arc::new(Mutex::new(OutputEventQueue::default()));
        let stdout_thread = spawn_stdout_reader(
            stdout,
            Arc::clone(&pending),
            Arc::clone(&exited),
            Arc::clone(&output_events),
            limits.clone(),
        );
        let stderr_thread = spawn_stderr_reader(
            stderr_pipe,
            Arc::clone(&stderr),
            limits.maximum_stderr_bytes,
        );
        Ok(Self {
            child: Mutex::new(Some(child)),
            writer: Mutex::new(Some(BufWriter::new(stdin))),
            pending,
            exited,
            stderr,
            output_events,
            stdout_thread: Mutex::new(Some(stdout_thread)),
            stderr_thread: Mutex::new(Some(stderr_thread)),
            limits: limits.clone(),
        })
    }

    fn fail_pending(&self, failure: PendingFailure) {
        fail_all_pending(&self.pending, failure);
    }
}

impl ExtensionHostProcess for StdioExtensionHostProcess {
    fn dispatch(
        &self,
        request: ExtensionHostRequest,
    ) -> Result<PendingHostRequest, ExtensionHostError> {
        request.validate(&self.limits)?;
        if self.has_exited() {
            return Err(ExtensionHostError::HostExited);
        }
        let bytes = serde_json::to_vec(&request)
            .map_err(|error| ExtensionHostError::InvalidProtocol(error.to_string()))?;
        let (waiter, sender) = PendingHostRequest::channel(request.context.request_id);
        {
            let mut pending = self
                .pending
                .lock()
                .map_err(|_| ExtensionHostError::HostExited)?;
            let control = matches!(
                &request.request,
                crate::HostRequestKind::Cancel(_)
                    | crate::HostRequestKind::Deactivate
                    | crate::HostRequestKind::Shutdown
            );
            reserve_pending(
                &mut pending,
                PendingEntry {
                    request,
                    sender,
                    control,
                },
                self.limits.maximum_in_flight_requests,
                self.limits.maximum_in_flight_control_requests,
            )?;
        }
        let write_result = self
            .writer
            .lock()
            .map_err(|_| ExtensionHostError::HostExited)
            .and_then(|mut writer| {
                let writer = writer.as_mut().ok_or(ExtensionHostError::HostExited)?;
                writer.write_all(&bytes)?;
                writer.write_all(b"\n")?;
                writer.flush()?;
                Ok(())
            });
        if write_result.is_err() {
            self.exited.store(true, Ordering::Release);
            self.fail_pending(PendingFailure::Transport);
            return Err(ExtensionHostError::HostExited);
        }
        Ok(waiter)
    }

    fn has_exited(&self) -> bool {
        if self.exited.load(Ordering::Acquire) {
            return true;
        }
        let Ok(mut child) = self.child.lock() else {
            return true;
        };
        let exited = child
            .as_mut()
            .and_then(|child| child.try_wait().ok())
            .flatten()
            .is_some();
        if exited {
            self.exited.store(true, Ordering::Release);
        }
        exited
    }

    fn terminate(&self) -> Result<(), ExtensionHostError> {
        self.exited.store(true, Ordering::Release);
        self.writer
            .lock()
            .map_err(|_| ExtensionHostError::HostExited)?
            .take();
        let mut child = self
            .child
            .lock()
            .map_err(|_| ExtensionHostError::HostExited)?;
        if let Some(mut child) = child.take() {
            if child
                .try_wait()
                .map_err(|_| ExtensionHostError::HostExited)?
                .is_none()
            {
                child.kill().map_err(|_| ExtensionHostError::HostExited)?;
            }
            child.wait().map_err(|_| ExtensionHostError::HostExited)?;
        }
        self.fail_pending(PendingFailure::Exited);
        join_thread(&self.stdout_thread);
        join_thread(&self.stderr_thread);
        Ok(())
    }

    fn stderr(&self) -> String {
        self.stderr
            .lock()
            .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
            .unwrap_or_default()
    }

    fn drain_output_events(&self) -> Vec<ExtensionHostOutputEvent> {
        self.output_events
            .lock()
            .map(|mut queue| {
                queue.bytes = 0;
                queue.events.drain(..).collect()
            })
            .unwrap_or_default()
    }
}

impl Drop for StdioExtensionHostProcess {
    fn drop(&mut self) {
        let _ = self.terminate();
    }
}

fn spawn_stdout_reader(
    stdout: ChildStdout,
    pending: Arc<Mutex<BTreeMap<u64, PendingEntry>>>,
    exited: Arc<AtomicBool>,
    output_events: Arc<Mutex<OutputEventQueue>>,
    limits: ExtensionHostLimits,
) -> JoinHandle<()> {
    thread::Builder::new()
        .name("zeta-editor-extension-host-stdout".into())
        .spawn(move || {
            let mut reader = BufReader::new(stdout);
            loop {
                let bytes = match read_bounded_line(&mut reader, limits.maximum_frame_bytes) {
                    Ok(Some(bytes)) => bytes,
                    Ok(None) => {
                        fail_all_pending(&pending, PendingFailure::Exited);
                        break;
                    }
                    Err(error) => {
                        fail_all_pending(&pending, PendingFailure::Protocol(error));
                        break;
                    }
                };
                let frame = match serde_json::from_slice::<ExtensionHostStdoutFrame>(&bytes) {
                    Ok(frame) => frame,
                    Err(error) => {
                        fail_all_pending(&pending, PendingFailure::Protocol(error.to_string()));
                        break;
                    }
                };
                let ExtensionHostStdoutFrame::Response(response) = frame else {
                    let ExtensionHostStdoutFrame::Output(event) = frame else {
                        unreachable!();
                    };
                    if let Err(error) = event.validate(&limits) {
                        fail_all_pending(&pending, PendingFailure::Protocol(error.to_string()));
                        break;
                    }
                    let Ok(mut queue) = output_events.lock() else {
                        fail_all_pending(&pending, PendingFailure::Transport);
                        break;
                    };
                    if queue.events.len() >= limits.maximum_output_event_count
                        || queue.bytes.saturating_add(bytes.len()) > limits.maximum_output_bytes
                    {
                        fail_all_pending(
                            &pending,
                            PendingFailure::Protocol("Output event quota exceeded".into()),
                        );
                        break;
                    }
                    queue.bytes += bytes.len();
                    queue.events.push_back(event);
                    continue;
                };
                let entry = pending
                    .lock()
                    .ok()
                    .and_then(|mut pending| pending.remove(&response.context.request_id));
                let Some(entry) = entry else {
                    fail_all_pending(
                        &pending,
                        PendingFailure::Protocol("response used an unknown request ID".into()),
                    );
                    break;
                };
                let response = response
                    .validate_for(&entry.request, &limits)
                    .map(|()| response)
                    .map_err(|error| PendingFailure::Protocol(error.to_string()));
                let invalid = response.is_err();
                let _ = entry.sender.send(response);
                if invalid {
                    fail_all_pending(
                        &pending,
                        PendingFailure::Protocol("invalid response correlation".into()),
                    );
                    break;
                }
            }
            exited.store(true, Ordering::Release);
        })
        .expect("extension host stdout reader thread must start")
}

fn spawn_stderr_reader(
    mut stderr: ChildStderr,
    captured: Arc<Mutex<Vec<u8>>>,
    maximum_bytes: usize,
) -> JoinHandle<()> {
    thread::Builder::new()
        .name("zeta-editor-extension-host-stderr".into())
        .spawn(move || {
            let mut buffer = [0_u8; 8192];
            while let Ok(read) = stderr.read(&mut buffer) {
                if read == 0 {
                    break;
                }
                let Ok(mut captured) = captured.lock() else {
                    continue;
                };
                let remaining = maximum_bytes.saturating_sub(captured.len());
                captured.extend_from_slice(&buffer[..read.min(remaining)]);
            }
        })
        .expect("extension host stderr reader thread must start")
}

pub(super) fn read_bounded_line<R: BufRead>(
    reader: &mut R,
    maximum_bytes: usize,
) -> Result<Option<Vec<u8>>, String> {
    let mut output = Vec::new();
    loop {
        let available = reader.fill_buf().map_err(|error| error.to_string())?;
        if available.is_empty() {
            return if output.is_empty() {
                Ok(None)
            } else {
                Err("protocol frame ended without a newline".into())
            };
        }
        let newline = available.iter().position(|byte| *byte == b'\n');
        let take = newline.map_or(available.len(), |position| position + 1);
        if output.len().saturating_add(take) > maximum_bytes.saturating_add(1) {
            return Err("protocol frame exceeds its byte limit".into());
        }
        output.extend_from_slice(&available[..take]);
        reader.consume(take);
        if newline.is_some() {
            output.pop();
            if output.last() == Some(&b'\r') {
                output.pop();
            }
            if output.len() > maximum_bytes {
                return Err("protocol frame exceeds its byte limit".into());
            }
            return Ok(Some(output));
        }
    }
}

fn fail_all_pending(pending: &Mutex<BTreeMap<u64, PendingEntry>>, failure: PendingFailure) {
    let Ok(mut pending) = pending.lock() else {
        return;
    };
    for (_, entry) in std::mem::take(&mut *pending) {
        let _ = entry.sender.send(Err(failure.clone()));
    }
}

fn join_thread(thread: &Mutex<Option<JoinHandle<()>>>) {
    if let Ok(mut thread) = thread.lock()
        && let Some(thread) = thread.take()
    {
        let _ = thread.join();
    }
}
