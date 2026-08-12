use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::mpsc;
use std::time::Duration;

use zeta_connectors::ConnectorConnectionState;
use zeta_connectors::ConnectorConnectionUpdate;
use zeta_connectors::ConnectorDefinition;
use zeta_connectors::ConnectorDefinitionDigest;
use zeta_connectors::ConnectorError;
use zeta_connectors::ConnectorId;
use zeta_connectors::ConnectorSnapshot;
use zeta_connectors::ConnectorSnapshotGeneration;

use crate::ConnectorAuthorityCommand;
use crate::ConnectorAuthorityError;
use crate::ConnectorAuthorityErrorKind;
use crate::ConnectorCommandDisposition;
use crate::ConnectorCommandRequest;
use crate::ConnectorCommandResult;
use crate::command::command_digest;

mod sqlite;

use sqlite::SqliteAuthority;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct CommandReceipt {
    pub expected_generation: ConnectorSnapshotGeneration,
    pub connector_id: ConnectorId,
    pub command_digest: String,
    pub result_generation: ConnectorSnapshotGeneration,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) enum AuthorityEvent {
    Begin {
        connector_id: ConnectorId,
        generation: zeta_connectors::ConnectorConnectionGeneration,
        definition_digest: ConnectorDefinitionDigest,
    },
    Complete {
        connector_id: ConnectorId,
        account: zeta_connectors::ConnectorAccount,
        definition_digest: ConnectorDefinitionDigest,
    },
    Unavailable {
        connector_id: ConnectorId,
        generation: zeta_connectors::ConnectorConnectionGeneration,
        reason: String,
    },
    Disconnect {
        connector_id: ConnectorId,
        generation: zeta_connectors::ConnectorConnectionGeneration,
    },
}

impl AuthorityEvent {
    pub(super) fn connector_id(&self) -> &ConnectorId {
        match self {
            Self::Begin { connector_id, .. }
            | Self::Complete { connector_id, .. }
            | Self::Unavailable { connector_id, .. }
            | Self::Disconnect { connector_id, .. } => connector_id,
        }
    }
}

struct AuthorityState {
    snapshot: ConnectorSnapshot,
    receipts: BTreeMap<String, CommandReceipt>,
}

enum Persistence {
    Memory,
    Sqlite(SqliteAuthority),
}

impl Persistence {
    fn persist(
        &self,
        event: &AuthorityEvent,
        result_generation: ConnectorSnapshotGeneration,
        command_id: &str,
        receipt: &CommandReceipt,
    ) -> Result<(), ConnectorAuthorityError> {
        match self {
            Self::Memory => Ok(()),
            Self::Sqlite(sqlite) => sqlite.persist(event, result_generation, command_id, receipt),
        }
    }
}

struct ConnectorAuthorityInner {
    state: Mutex<AuthorityState>,
    persistence: Persistence,
    subscribers: Mutex<Vec<mpsc::Sender<ConnectorSnapshotGeneration>>>,
}

/// Durable authority for Connector connection state and retry-safe mutation receipts.
///
/// Definitions are supplied by validated sources. This authority persists account projections and
/// credential references, never credential bytes or live MCP state.
#[derive(Clone)]
pub struct ConnectorAuthority {
    inner: Arc<ConnectorAuthorityInner>,
}

impl ConnectorAuthority {
    pub fn in_memory(
        definitions: impl IntoIterator<Item = ConnectorDefinition>,
    ) -> Result<Self, ConnectorAuthorityError> {
        let snapshot = ConnectorSnapshot::new(ConnectorSnapshotGeneration::new(1), definitions)
            .map_err(domain_error)?;
        Ok(Self::from_parts(
            snapshot,
            BTreeMap::new(),
            Persistence::Memory,
        ))
    }

    pub fn open_sqlite(
        path: impl AsRef<Path>,
        definitions: impl IntoIterator<Item = ConnectorDefinition>,
    ) -> Result<Self, ConnectorAuthorityError> {
        let definitions = definitions.into_iter().collect::<Vec<_>>();
        let loaded = SqliteAuthority::open(path.as_ref(), definitions)?;
        Ok(Self::from_parts(
            loaded.snapshot,
            loaded.receipts,
            Persistence::Sqlite(loaded.authority),
        ))
    }

    fn from_parts(
        snapshot: ConnectorSnapshot,
        receipts: BTreeMap<String, CommandReceipt>,
        persistence: Persistence,
    ) -> Self {
        Self {
            inner: Arc::new(ConnectorAuthorityInner {
                state: Mutex::new(AuthorityState { snapshot, receipts }),
                persistence,
                subscribers: Mutex::new(Vec::new()),
            }),
        }
    }

    pub fn snapshot(&self) -> ConnectorSnapshot {
        self.inner
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .snapshot
            .clone()
    }

    pub fn subscribe(&self) -> ConnectorAuthoritySubscription {
        let (sender, receiver) = mpsc::channel();
        self.inner
            .subscribers
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push(sender);
        ConnectorAuthoritySubscription { receiver }
    }

    pub fn apply(
        &self,
        request: ConnectorCommandRequest,
    ) -> Result<ConnectorCommandResult, ConnectorAuthorityError> {
        let request_digest = command_digest(&request);
        let mut state = self
            .inner
            .state
            .lock()
            .map_err(|_| persistence_error("connector authority lock poisoned"))?;
        if let Some(receipt) = state.receipts.get(request.command_id.as_str()) {
            if receipt.expected_generation != request.expected_generation
                || receipt.connector_id != request.connector_id
                || receipt.command_digest != request_digest
            {
                return Err(ConnectorAuthorityError::new(
                    ConnectorAuthorityErrorKind::CommandConflict,
                    "connector command ID was already used for a different request",
                ));
            }
            return Ok(ConnectorCommandResult {
                generation: receipt.result_generation,
                disposition: ConnectorCommandDisposition::Replayed,
            });
        }
        if state.snapshot.generation() != request.expected_generation {
            return Err(ConnectorAuthorityError::new(
                ConnectorAuthorityErrorKind::GenerationConflict,
                format!(
                    "connector generation conflict: expected {}, actual {}",
                    request.expected_generation.get(),
                    state.snapshot.generation().get()
                ),
            ));
        }
        let result_generation = next_generation(state.snapshot.generation())?;
        let event = event_for_request(&state.snapshot, &request)?;
        let next_snapshot = apply_event(&state.snapshot, result_generation, &event)?;
        let receipt = CommandReceipt {
            expected_generation: request.expected_generation,
            connector_id: request.connector_id,
            command_digest: request_digest,
            result_generation,
        };
        self.inner.persistence.persist(
            &event,
            result_generation,
            request.command_id.as_str(),
            &receipt,
        )?;
        state
            .receipts
            .insert(request.command_id.as_str().to_string(), receipt);
        state.snapshot = next_snapshot;
        drop(state);
        self.publish(result_generation);
        Ok(ConnectorCommandResult {
            generation: result_generation,
            disposition: ConnectorCommandDisposition::Updated,
        })
    }

    /// Checks the live generation and definition digest used by a prepared Connector tool call.
    pub fn authorizes(
        &self,
        connector_id: &ConnectorId,
        connection_generation: zeta_connectors::ConnectorConnectionGeneration,
        definition_digest: &ConnectorDefinitionDigest,
    ) -> bool {
        let state = self
            .inner
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let Some(entry) = state.snapshot.entry(connector_id) else {
            return false;
        };
        entry.definition().digest() == *definition_digest
            && entry.connection().generation() == connection_generation
            && matches!(
                entry.connection().state(),
                ConnectorConnectionState::Connected(_)
            )
    }

    /// Runs an invocation while holding the same authority lock used by disconnect commits.
    ///
    /// The operation starts only when the exact connected generation and definition digest are
    /// still live. A concurrent disconnect therefore either commits first and rejects the call,
    /// or waits for an already-authorized call to finish before revoking subsequent calls.
    pub fn with_authorized_invocation<T>(
        &self,
        connector_id: &ConnectorId,
        connection_generation: zeta_connectors::ConnectorConnectionGeneration,
        definition_digest: &ConnectorDefinitionDigest,
        operation: impl FnOnce() -> T,
    ) -> Option<T> {
        let state = self
            .inner
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let entry = state.snapshot.entry(connector_id)?;
        if entry.definition().digest() != *definition_digest
            || entry.connection().generation() != connection_generation
            || !matches!(
                entry.connection().state(),
                ConnectorConnectionState::Connected(_)
            )
        {
            return None;
        }
        Some(operation())
    }

    fn publish(&self, generation: ConnectorSnapshotGeneration) {
        self.inner
            .subscribers
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .retain(|subscriber| subscriber.send(generation).is_ok());
    }
}

/// Blocking subscription to committed Connector authority generations.
pub struct ConnectorAuthoritySubscription {
    receiver: mpsc::Receiver<ConnectorSnapshotGeneration>,
}

impl ConnectorAuthoritySubscription {
    pub fn try_recv(&self) -> Result<ConnectorSnapshotGeneration, mpsc::TryRecvError> {
        self.receiver.try_recv()
    }

    pub fn recv_timeout(
        &self,
        timeout: Duration,
    ) -> Result<ConnectorSnapshotGeneration, mpsc::RecvTimeoutError> {
        self.receiver.recv_timeout(timeout)
    }
}

pub(super) fn event_for_request(
    snapshot: &ConnectorSnapshot,
    request: &ConnectorCommandRequest,
) -> Result<AuthorityEvent, ConnectorAuthorityError> {
    let entry = snapshot.entry(&request.connector_id).ok_or_else(|| {
        ConnectorAuthorityError::new(
            ConnectorAuthorityErrorKind::InvalidCommand,
            "connector command targets an unavailable connector",
        )
    })?;
    Ok(match &request.command {
        ConnectorAuthorityCommand::BeginConnect { generation } => AuthorityEvent::Begin {
            connector_id: request.connector_id.clone(),
            generation: *generation,
            definition_digest: entry.definition().digest(),
        },
        ConnectorAuthorityCommand::CompleteConnect { account } => AuthorityEvent::Complete {
            connector_id: request.connector_id.clone(),
            account: account.clone(),
            definition_digest: entry.definition().digest(),
        },
        ConnectorAuthorityCommand::MarkUnavailable { generation, reason } => {
            AuthorityEvent::Unavailable {
                connector_id: request.connector_id.clone(),
                generation: *generation,
                reason: reason.clone(),
            }
        }
        ConnectorAuthorityCommand::Disconnect { generation } => AuthorityEvent::Disconnect {
            connector_id: request.connector_id.clone(),
            generation: *generation,
        },
    })
}

pub(super) fn apply_event(
    snapshot: &ConnectorSnapshot,
    result_generation: ConnectorSnapshotGeneration,
    event: &AuthorityEvent,
) -> Result<ConnectorSnapshot, ConnectorAuthorityError> {
    let update = match event {
        AuthorityEvent::Begin { generation, .. } => ConnectorConnectionUpdate::Begin {
            generation: *generation,
        },
        AuthorityEvent::Complete { account, .. } => ConnectorConnectionUpdate::Connected {
            account: account.clone(),
        },
        AuthorityEvent::Unavailable {
            generation, reason, ..
        } => ConnectorConnectionUpdate::Unavailable {
            generation: *generation,
            reason: reason.clone(),
        },
        AuthorityEvent::Disconnect { generation, .. } => ConnectorConnectionUpdate::Disconnect {
            generation: *generation,
        },
    };
    snapshot
        .with_connection_update(result_generation, event.connector_id(), update)
        .map_err(domain_error)
}

fn next_generation(
    generation: ConnectorSnapshotGeneration,
) -> Result<ConnectorSnapshotGeneration, ConnectorAuthorityError> {
    generation
        .get()
        .checked_add(1)
        .map(ConnectorSnapshotGeneration::new)
        .ok_or_else(|| persistence_error("connector snapshot generation exhausted"))
}

pub(super) fn domain_error(error: ConnectorError) -> ConnectorAuthorityError {
    ConnectorAuthorityError::new(ConnectorAuthorityErrorKind::Domain, error.to_string())
}

pub(super) fn persistence_error(message: impl Into<String>) -> ConnectorAuthorityError {
    ConnectorAuthorityError::new(ConnectorAuthorityErrorKind::Persistence, message)
}
