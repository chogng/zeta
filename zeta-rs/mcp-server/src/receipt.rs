use crate::agent::{AgentCallError, AgentOutcome, InvocationFingerprint};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use zeta_protocol::{SessionId, ThreadId};

const RECEIPT_VERSION: u32 = 1;

pub(crate) enum BeginInvocation {
    Execute,
    Replay(AgentOutcome),
}

pub(crate) struct ReceiptStore {
    path: Option<PathBuf>,
    inner: Mutex<ReceiptState>,
}

struct ReceiptState {
    document: ReceiptDocument,
    active: BTreeSet<(String, String)>,
}

#[derive(Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct ReceiptDocument {
    version: u32,
    principals: BTreeMap<String, PrincipalReceipts>,
}

#[derive(Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct PrincipalReceipts {
    invocations: BTreeMap<String, InvocationReceipt>,
    threads: BTreeMap<ThreadId, SessionId>,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct InvocationReceipt {
    fingerprint: String,
    state: InvocationReceiptState,
}

#[derive(Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
enum InvocationReceiptState {
    Running,
    Finished { outcome: AgentOutcome },
}

impl ReceiptStore {
    pub(crate) fn open(path: impl Into<PathBuf>) -> Result<Self, AgentCallError> {
        let path = path.into();
        let document = if path.exists() {
            let bytes = fs::read(&path).map_err(receipt_error)?;
            let document: ReceiptDocument =
                serde_json::from_slice(&bytes).map_err(receipt_error)?;
            if document.version != RECEIPT_VERSION {
                return Err(AgentCallError::AppServer(format!(
                    "unsupported MCP receipt version {}",
                    document.version
                )));
            }
            document
        } else {
            ReceiptDocument {
                version: RECEIPT_VERSION,
                ..ReceiptDocument::default()
            }
        };
        Ok(Self {
            path: Some(path),
            inner: Mutex::new(ReceiptState {
                document,
                active: BTreeSet::new(),
            }),
        })
    }

    #[cfg(test)]
    pub(crate) fn memory() -> Self {
        Self {
            path: None,
            inner: Mutex::new(ReceiptState {
                document: ReceiptDocument {
                    version: RECEIPT_VERSION,
                    ..ReceiptDocument::default()
                },
                active: BTreeSet::new(),
            }),
        }
    }

    pub(crate) fn begin(
        &self,
        principal: &str,
        invocation_id: &str,
        fingerprint: InvocationFingerprint,
    ) -> Result<BeginInvocation, AgentCallError> {
        let key = (principal.to_string(), invocation_id.to_string());
        let mut state = self.inner.lock().map_err(|_| receipt_lock_error())?;
        if state.active.contains(&key) {
            return Err(AgentCallError::InvocationInProgress);
        }
        let fingerprint = fingerprint.encode();
        let existing = state
            .document
            .principals
            .get(principal)
            .and_then(|receipts| receipts.invocations.get(invocation_id));
        match existing {
            Some(receipt) if receipt.fingerprint != fingerprint => {
                return Err(AgentCallError::InvocationConflict);
            }
            Some(InvocationReceipt {
                state: InvocationReceiptState::Finished { outcome },
                ..
            }) => return Ok(BeginInvocation::Replay(outcome.clone())),
            Some(InvocationReceipt {
                state: InvocationReceiptState::Running,
                ..
            }) => {}
            None => {
                state
                    .document
                    .principals
                    .entry(principal.into())
                    .or_default()
                    .invocations
                    .insert(
                        invocation_id.into(),
                        InvocationReceipt {
                            fingerprint,
                            state: InvocationReceiptState::Running,
                        },
                    );
                persist(self.path.as_deref(), &state.document)?;
            }
        }
        state.active.insert(key);
        Ok(BeginInvocation::Execute)
    }

    pub(crate) fn finish(
        &self,
        principal: &str,
        invocation_id: &str,
        fingerprint: InvocationFingerprint,
        result: Result<AgentOutcome, AgentCallError>,
    ) -> Result<AgentOutcome, AgentCallError> {
        let mut state = self.inner.lock().map_err(|_| receipt_lock_error())?;
        state
            .active
            .remove(&(principal.to_string(), invocation_id.to_string()));
        let receipts = state
            .document
            .principals
            .entry(principal.into())
            .or_default();
        match &result {
            Ok(outcome) if outcome.is_terminal() => {
                receipts.invocations.insert(
                    invocation_id.into(),
                    InvocationReceipt {
                        fingerprint: fingerprint.encode(),
                        state: InvocationReceiptState::Finished {
                            outcome: outcome.clone(),
                        },
                    },
                );
            }
            Ok(_) => {}
            Err(_) => {
                receipts.invocations.remove(invocation_id);
            }
        }
        persist(self.path.as_deref(), &state.document)?;
        result
    }

    pub(crate) fn bind_thread(
        &self,
        principal: &str,
        thread_id: ThreadId,
        session_id: SessionId,
    ) -> Result<(), AgentCallError> {
        let mut state = self.inner.lock().map_err(|_| receipt_lock_error())?;
        let threads = &mut state
            .document
            .principals
            .entry(principal.into())
            .or_default()
            .threads;
        match threads.get(&thread_id) {
            Some(existing) if existing != &session_id => {
                return Err(AgentCallError::AppServer(
                    "durable Thread binding conflicts with the App Server result".into(),
                ));
            }
            Some(_) => return Ok(()),
            None => {
                threads.insert(thread_id, session_id);
            }
        }
        persist(self.path.as_deref(), &state.document)
    }

    pub(crate) fn session_for_thread(
        &self,
        principal: &str,
        thread_id: &ThreadId,
    ) -> Result<Option<SessionId>, AgentCallError> {
        let state = self.inner.lock().map_err(|_| receipt_lock_error())?;
        Ok(state
            .document
            .principals
            .get(principal)
            .and_then(|receipts| receipts.threads.get(thread_id))
            .cloned())
    }
}

fn persist(path: Option<&Path>, document: &ReceiptDocument) -> Result<(), AgentCallError> {
    let Some(path) = path else {
        return Ok(());
    };
    let parent = path
        .parent()
        .ok_or_else(|| AgentCallError::AppServer("receipt path has no parent".into()))?;
    fs::create_dir_all(parent).map_err(receipt_error)?;
    let bytes = serde_json::to_vec(document).map_err(receipt_error)?;
    let temporary = path.with_extension(format!("tmp-{}", std::process::id()));
    let mut options = fs::OpenOptions::new();
    options.create(true).truncate(true).write(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(&temporary).map_err(receipt_error)?;
    file.write_all(&bytes).map_err(receipt_error)?;
    file.sync_all().map_err(receipt_error)?;
    fs::rename(&temporary, path).map_err(receipt_error)?;
    sync_parent(parent)
}

#[cfg(unix)]
fn sync_parent(parent: &Path) -> Result<(), AgentCallError> {
    fs::File::open(parent)
        .and_then(|directory| directory.sync_all())
        .map_err(receipt_error)
}

#[cfg(not(unix))]
fn sync_parent(_parent: &Path) -> Result<(), AgentCallError> {
    Ok(())
}

fn receipt_lock_error() -> AgentCallError {
    AgentCallError::AppServer("MCP receipt lock poisoned".into())
}

fn receipt_error(error: impl std::fmt::Display) -> AgentCallError {
    AgentCallError::AppServer(format!("MCP receipt store failed: {error}"))
}

#[cfg(test)]
#[path = "receipt_tests.rs"]
mod tests;
