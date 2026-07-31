use crate::terminal_profiles::TerminalProfileCatalog;
use base64::Engine;
use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use tokio::runtime::Runtime;
use zeta_app_server_protocol::protocol::terminal::{
    TerminalCreateParams, TerminalCreateResult, TerminalOutputChunk, TerminalProfile,
    TerminalReadParams, TerminalReadResult, TerminalResizeParams, TerminalWriteParams,
};
use zeta_utils_pty::{ProcessHandle, SpawnedProcess, TerminalSize, spawn_pty_process};
use zeta_workspace::{TrustedWorkspace, WorkspaceCapability};

const MAX_ACTIVE_TERMINALS: usize = 16;
const MAX_OUTPUT_BYTES: usize = 1024 * 1024;
const MAX_INPUT_BYTES: usize = 64 * 1024;

/// Owns connection-scoped interactive PTY processes rooted at one trusted workspace.
pub(crate) struct TerminalService {
    workspace: RwLock<TrustedWorkspace>,
    next_terminal_id: AtomicU64,
    sessions: Mutex<HashMap<String, TerminalSession>>,
    runtime: Runtime,
    profiles: TerminalProfileCatalog,
}

impl TerminalService {
    pub(crate) fn new(workspace: TrustedWorkspace) -> Result<Self, TerminalError> {
        validate_workspace_capability(&workspace)?;
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .thread_name("zeta-terminal")
            .build()
            .map_err(|_| TerminalError::OperationFailed)?;
        Ok(Self {
            workspace: RwLock::new(workspace),
            next_terminal_id: AtomicU64::new(1),
            sessions: Mutex::new(HashMap::new()),
            runtime,
            profiles: TerminalProfileCatalog::discover(),
        })
    }

    pub(crate) fn switch_workspace(
        &self,
        workspace: TrustedWorkspace,
    ) -> Result<(), TerminalError> {
        validate_workspace_capability(&workspace)?;
        *self
            .workspace
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = workspace;
        Ok(())
    }

    pub(crate) fn profiles(&self) -> Vec<TerminalProfile> {
        self.profiles.list()
    }

    pub(crate) fn create(
        &self,
        owner_connection_id: u64,
        params: TerminalCreateParams,
    ) -> Result<TerminalCreateResult, TerminalError> {
        self.ensure_trusted()?;
        validate_size(params.rows, params.cols)?;
        let profile = self
            .profiles
            .resolve(&params.profile)
            .ok_or(TerminalError::InvalidInput)?;
        let mut sessions = self.sessions.lock().map_err(|_| TerminalError::Busy)?;
        if sessions.len() >= MAX_ACTIVE_TERMINALS {
            return Err(TerminalError::Busy);
        }
        let workspace_root = self
            .workspace
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .root()
            .canonical_path()
            .to_path_buf();
        let spawned = self
            .runtime
            .block_on(spawn_pty_process(
                &profile.program,
                &profile.args,
                &workspace_root,
                self.profiles.environment(),
                &None,
                TerminalSize {
                    rows: params.rows,
                    cols: params.cols,
                },
                &[],
            ))
            .map_err(|_| TerminalError::OperationFailed)?;
        let terminal_id = format!(
            "terminal-{:x}",
            self.next_terminal_id.fetch_add(1, Ordering::Relaxed)
        );
        let state = Arc::new(Mutex::new(TerminalState::default()));
        let process = spawn_output_drainers(&self.runtime, spawned, state.clone());
        sessions.insert(
            terminal_id.clone(),
            TerminalSession {
                owner_connection_id,
                process,
                state,
            },
        );
        Ok(TerminalCreateResult {
            terminal_id,
            profile: profile.dto(),
        })
    }

    pub(crate) fn write(
        &self,
        owner_connection_id: u64,
        params: TerminalWriteParams,
    ) -> Result<(), TerminalError> {
        self.ensure_trusted()?;
        if params.data.is_empty() || params.data.len() > MAX_INPUT_BYTES {
            return Err(TerminalError::InvalidInput);
        }
        let sessions = self.owned_sessions(owner_connection_id, &params.terminal_id)?;
        let writer = sessions
            .get(&params.terminal_id)
            .expect("terminal ownership was just validated")
            .process
            .writer_sender();
        drop(sessions);
        self.runtime
            .block_on(writer.send(params.data.into_bytes()))
            .map_err(|_| TerminalError::OperationFailed)
    }

    pub(crate) fn resize(
        &self,
        owner_connection_id: u64,
        params: TerminalResizeParams,
    ) -> Result<(), TerminalError> {
        self.ensure_trusted()?;
        validate_size(params.rows, params.cols)?;
        let sessions = self.owned_sessions(owner_connection_id, &params.terminal_id)?;
        sessions
            .get(&params.terminal_id)
            .expect("terminal ownership was just validated")
            .process
            .resize(TerminalSize {
                rows: params.rows,
                cols: params.cols,
            })
            .map_err(|_| TerminalError::OperationFailed)
    }

    pub(crate) fn read(
        &self,
        owner_connection_id: u64,
        params: TerminalReadParams,
    ) -> Result<TerminalReadResult, TerminalError> {
        self.ensure_trusted()?;
        if params.max_chunks == 0 || params.max_chunks > 128 {
            return Err(TerminalError::InvalidInput);
        }
        let sessions = self.owned_sessions(owner_connection_id, &params.terminal_id)?;
        let state = sessions
            .get(&params.terminal_id)
            .expect("terminal ownership was just validated")
            .state
            .clone();
        drop(sessions);
        let state = state.lock().map_err(|_| TerminalError::Busy)?;
        if params.after_sequence > state.next_sequence {
            return Err(TerminalError::InvalidInput);
        }
        Ok(read_state(&params, &state))
    }

    pub(crate) fn close(
        &self,
        owner_connection_id: u64,
        terminal_id: &str,
    ) -> Result<(), TerminalError> {
        let mut sessions = self.sessions.lock().map_err(|_| TerminalError::Busy)?;
        let session = sessions.get(terminal_id).ok_or(TerminalError::NotFound)?;
        if session.owner_connection_id != owner_connection_id {
            return Err(TerminalError::NotOwner);
        }
        sessions
            .remove(terminal_id)
            .expect("terminal existed immediately before removal")
            .process
            .request_terminate();
        Ok(())
    }

    pub(crate) fn close_owner(&self, owner_connection_id: u64) {
        let Ok(mut sessions) = self.sessions.lock() else {
            return;
        };
        sessions.retain(|_, session| {
            if session.owner_connection_id == owner_connection_id {
                session.process.request_terminate();
                false
            } else {
                true
            }
        });
    }

    pub(crate) fn terminate_all(&self) {
        let Ok(mut sessions) = self.sessions.lock() else {
            return;
        };
        for (_, session) in sessions.drain() {
            session.process.request_terminate();
        }
    }

    fn ensure_trusted(&self) -> Result<(), TerminalError> {
        self.workspace
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .ensure_active()
            .map_err(|_| TerminalError::OperationFailed)
    }

    fn owned_sessions(
        &self,
        owner_connection_id: u64,
        terminal_id: &str,
    ) -> Result<std::sync::MutexGuard<'_, HashMap<String, TerminalSession>>, TerminalError> {
        let sessions = self.sessions.lock().map_err(|_| TerminalError::Busy)?;
        let session = sessions.get(terminal_id).ok_or(TerminalError::NotFound)?;
        if session.owner_connection_id != owner_connection_id {
            return Err(TerminalError::NotOwner);
        }
        Ok(sessions)
    }
}

fn validate_workspace_capability(workspace: &TrustedWorkspace) -> Result<(), TerminalError> {
    if workspace.capability() == WorkspaceCapability::ExecuteProcess
        && workspace.ensure_active().is_ok()
    {
        Ok(())
    } else {
        Err(TerminalError::OperationFailed)
    }
}

struct TerminalSession {
    owner_connection_id: u64,
    process: Arc<ProcessHandle>,
    state: Arc<Mutex<TerminalState>>,
}

#[derive(Default)]
struct TerminalState {
    chunks: VecDeque<BufferedOutput>,
    next_sequence: u64,
    output_bytes: usize,
    exited: bool,
    output_closed: bool,
    exit_code: Option<i32>,
}

struct BufferedOutput {
    sequence: u64,
    bytes: Vec<u8>,
}

fn spawn_output_drainers(
    runtime: &Runtime,
    spawned: SpawnedProcess,
    state: Arc<Mutex<TerminalState>>,
) -> Arc<ProcessHandle> {
    let SpawnedProcess {
        session,
        mut stdout_rx,
        stderr_rx: _,
        exit_rx,
    } = spawned;
    let session = Arc::new(session);
    let output_state = state.clone();
    runtime.spawn(async move {
        while let Some(bytes) = stdout_rx.recv().await {
            if let Ok(mut state) = output_state.lock() {
                push_output(&mut state, bytes);
            }
        }
        if let Ok(mut state) = output_state.lock() {
            state.output_closed = true;
            state.exited = state.exit_code.is_some();
        }
    });
    let exit_session = session.clone();
    runtime.spawn(async move {
        let exit_code = exit_rx.await.unwrap_or(-1);
        exit_session.release_pty_handles_after_exit();
        if let Ok(mut state) = state.lock() {
            state.exit_code = Some(exit_code);
            state.exited = state.output_closed;
        }
    });
    session
}

fn push_output(state: &mut TerminalState, bytes: Vec<u8>) {
    if bytes.is_empty() {
        return;
    }
    state.next_sequence = state.next_sequence.saturating_add(1);
    state.output_bytes = state.output_bytes.saturating_add(bytes.len());
    state.chunks.push_back(BufferedOutput {
        sequence: state.next_sequence,
        bytes,
    });
    while state.output_bytes > MAX_OUTPUT_BYTES {
        let Some(removed) = state.chunks.pop_front() else {
            break;
        };
        state.output_bytes = state.output_bytes.saturating_sub(removed.bytes.len());
    }
}

fn read_state(params: &TerminalReadParams, state: &TerminalState) -> TerminalReadResult {
    let oldest_sequence = state
        .chunks
        .front()
        .map_or(state.next_sequence.saturating_add(1), |chunk| {
            chunk.sequence
        });
    let requested_sequence = params.after_sequence.saturating_add(1);
    let output_gap = requested_sequence < oldest_sequence;
    let first_sequence = if output_gap {
        oldest_sequence
    } else {
        requested_sequence
    };
    let chunks = state
        .chunks
        .iter()
        .filter(|chunk| chunk.sequence >= first_sequence)
        .take(params.max_chunks)
        .map(|chunk| TerminalOutputChunk {
            sequence: chunk.sequence,
            data_base64: base64::engine::general_purpose::STANDARD.encode(&chunk.bytes),
        })
        .collect::<Vec<_>>();
    let next_sequence = chunks.last().map_or_else(
        || {
            if output_gap {
                state.next_sequence
            } else {
                params.after_sequence
            }
        },
        |chunk| chunk.sequence,
    );
    TerminalReadResult {
        terminal_id: params.terminal_id.clone(),
        chunks,
        next_sequence,
        output_gap,
        exited: state.exited,
        exit_code: state.exit_code,
    }
}

fn validate_size(rows: u16, cols: u16) -> Result<(), TerminalError> {
    if rows == 0 || rows > 512 || cols == 0 || cols > 512 {
        return Err(TerminalError::InvalidInput);
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum TerminalError {
    InvalidInput,
    NotFound,
    NotOwner,
    Busy,
    OperationFailed,
}

#[cfg(test)]
#[path = "terminal_service_tests.rs"]
mod tests;
