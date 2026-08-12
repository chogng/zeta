use super::request::require_history_boundary;
use std::collections::BTreeMap;
use zeta_app_server_client::AppServerClient;
use zeta_app_server_client::ClientError;
use zeta_app_server_client::JsonRpcTransport;
use zeta_app_server_protocol::protocol::session::MAX_THREAD_SNAPSHOT_TURNS;
use zeta_app_server_protocol::protocol::session::SessionThreadReadParams;
use zeta_app_server_protocol::protocol::session::SessionThreadSubscribeParams;
use zeta_app_server_protocol::protocol::session::SessionThreadUnsubscribeParams;
use zeta_app_server_protocol::protocol::session::ThreadHistoryBoundary;
use zeta_app_server_protocol::protocol::session::ThreadSnapshotHistory;
use zeta_protocol::SessionId;
use zeta_protocol::StreamInstanceId;
use zeta_protocol::Thread;
use zeta_protocol::ThreadId;
use zeta_protocol::ThreadUpdateEnvelope;

/// Tracks the server subscription and last canonical snapshot for the active Thread.
///
/// It does not apply Thread events locally. A newer durable sequence asks the caller to read a
/// fresh authoritative snapshot, which keeps the product reducer in Core.
#[derive(Clone, Debug)]
pub(crate) struct ThreadSubscription {
    session_id: SessionId,
    thread_id: ThreadId,
    confirmed_sequence: u64,
    history_turn_limit: u32,
    oldest_turn_id: Option<zeta_protocol::TurnId>,
    has_older_turns: bool,
    stream_sequences: BTreeMap<StreamInstanceId, u64>,
}

const HISTORY_PAGE_TURNS: u32 = 50;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ThreadUpdateDisposition {
    ApplyTransient,
    ApplyTransientAfterReset,
    Ignore,
    RefreshSnapshot,
    ResetTransientAndRefreshSnapshot,
}

pub(crate) enum ThreadSwitch {
    Complete {
        snapshot: Thread,
    },
    StaleSubscription {
        snapshot: Thread,
        error: ClientError,
    },
}

impl ThreadSubscription {
    pub(crate) fn start<T>(
        client: &mut AppServerClient<T>,
        session_id: &SessionId,
        thread_id: &ThreadId,
    ) -> Result<(Self, Thread), ClientError>
    where
        T: JsonRpcTransport,
    {
        let result = client.subscribe_session_thread(SessionThreadSubscribeParams {
            session_id: session_id.clone(),
            thread_id: thread_id.clone(),
            after_sequence: 0,
            history: Some(ThreadSnapshotHistory::Latest {
                turn_limit: HISTORY_PAGE_TURNS,
            }),
        })?;
        validate_snapshot_scope(&result.thread, session_id, thread_id)?;
        validate_update_scopes(&result.updates, session_id, thread_id)?;

        let (snapshot, boundary) = if result
            .updates
            .iter()
            .any(|update| update.durable_sequence > result.thread.sequence)
        {
            let read = client.read_session_thread(SessionThreadReadParams {
                session_id: session_id.clone(),
                thread_id: thread_id.clone(),
                history: Some(ThreadSnapshotHistory::Latest {
                    turn_limit: HISTORY_PAGE_TURNS,
                }),
            })?;
            let boundary = require_history_boundary(read.history)?;
            (read.thread, boundary)
        } else {
            let boundary = require_history_boundary(result.history)?;
            (result.thread, boundary)
        };
        validate_snapshot_scope(&snapshot, session_id, thread_id)?;

        Ok((
            Self::from_snapshot_with_boundary(&snapshot, HISTORY_PAGE_TURNS, Some(boundary)),
            snapshot,
        ))
    }

    pub(crate) fn switch<T>(
        &mut self,
        client: &mut AppServerClient<T>,
        session_id: &SessionId,
        thread_id: &ThreadId,
    ) -> Result<ThreadSwitch, ClientError>
    where
        T: JsonRpcTransport,
    {
        if self.session_id == *session_id && self.thread_id == *thread_id {
            let result = client.read_session_thread(SessionThreadReadParams {
                session_id: session_id.clone(),
                thread_id: thread_id.clone(),
                history: Some(self.history()),
            })?;
            let snapshot = result.thread;
            let boundary = require_history_boundary(result.history)?;
            validate_snapshot_scope(&snapshot, session_id, thread_id)?;
            *self = Self::from_snapshot_with_boundary(
                &snapshot,
                self.history_turn_limit,
                Some(boundary),
            );
            return Ok(ThreadSwitch::Complete { snapshot });
        }

        let previous_session_id = self.session_id.clone();
        let previous_thread_id = self.thread_id.clone();
        let (next, snapshot) = Self::start(client, session_id, thread_id)?;
        *self = next;
        let cleanup = client.unsubscribe_session_thread(SessionThreadUnsubscribeParams {
            session_id: previous_session_id,
            thread_id: previous_thread_id,
        });
        match cleanup {
            Ok(()) => Ok(ThreadSwitch::Complete { snapshot }),
            Err(error) => Ok(ThreadSwitch::StaleSubscription { snapshot, error }),
        }
    }

    pub(crate) fn classify_update(
        &mut self,
        update: &ThreadUpdateEnvelope,
    ) -> ThreadUpdateDisposition {
        if update.thread_id != self.thread_id {
            return ThreadUpdateDisposition::Ignore;
        }
        if update.session_id != self.session_id || update.durable_sequence > self.confirmed_sequence
        {
            return ThreadUpdateDisposition::RefreshSnapshot;
        }
        if update.durable_sequence < self.confirmed_sequence
            || matches!(update.update, zeta_protocol::ThreadUpdate::Committed { .. })
        {
            return ThreadUpdateDisposition::Ignore;
        }
        let Some(cursor) = &update.stream_cursor else {
            return ThreadUpdateDisposition::Ignore;
        };
        if let Some(sequence) = self.stream_sequences.get_mut(&cursor.stream_instance_id) {
            if cursor.sequence <= *sequence {
                return ThreadUpdateDisposition::Ignore;
            }
            let continuous = cursor.sequence == sequence.saturating_add(1);
            *sequence = cursor.sequence;
            return if continuous {
                ThreadUpdateDisposition::ApplyTransient
            } else {
                ThreadUpdateDisposition::ResetTransientAndRefreshSnapshot
            };
        }

        self.stream_sequences.clear();
        self.stream_sequences
            .insert(cursor.stream_instance_id.clone(), cursor.sequence);
        if cursor.sequence == 1 {
            ThreadUpdateDisposition::ApplyTransientAfterReset
        } else {
            ThreadUpdateDisposition::ResetTransientAndRefreshSnapshot
        }
    }

    pub(crate) fn confirm_sequence(&mut self, sequence: u64) {
        if sequence > self.confirmed_sequence {
            self.confirmed_sequence = sequence;
        }
    }

    pub(crate) fn history(&self) -> ThreadSnapshotHistory {
        ThreadSnapshotHistory::Latest {
            turn_limit: self.history_turn_limit,
        }
    }

    /// Applies the cursor and sequence returned with a latest-history snapshot.
    pub(crate) fn apply_latest_snapshot(
        &mut self,
        snapshot: &Thread,
        boundary: ThreadHistoryBoundary,
    ) {
        self.confirm_sequence(snapshot.sequence);
        self.oldest_turn_id = boundary
            .oldest_turn_id
            .clone()
            .or_else(|| snapshot.turns.first().map(|turn| turn.turn_id.clone()));
        self.has_older_turns = boundary.has_older_turns;
    }

    pub(crate) fn older_history(&self) -> Option<ThreadSnapshotHistory> {
        self.has_older_turns
            .then(|| self.oldest_turn_id.clone())
            .flatten()
            .map(|turn_id| ThreadSnapshotHistory::Before {
                turn_id,
                turn_limit: HISTORY_PAGE_TURNS,
            })
    }

    pub(crate) fn apply_history_page(
        &mut self,
        snapshot: &Thread,
        boundary: ThreadHistoryBoundary,
    ) {
        self.history_turn_limit = self
            .history_turn_limit
            .saturating_add(snapshot.turns.len() as u32)
            .min(MAX_THREAD_SNAPSHOT_TURNS);
        self.oldest_turn_id = boundary
            .oldest_turn_id
            .clone()
            .or_else(|| snapshot.turns.first().map(|turn| turn.turn_id.clone()));
        self.has_older_turns = boundary.has_older_turns;
    }

    #[cfg(test)]
    pub(crate) fn expand_history(&mut self) {
        self.history_turn_limit = self
            .history_turn_limit
            .saturating_add(HISTORY_PAGE_TURNS)
            .min(MAX_THREAD_SNAPSHOT_TURNS);
    }

    #[cfg(test)]
    fn from_snapshot(snapshot: &Thread, history_turn_limit: u32) -> Self {
        Self::from_snapshot_with_boundary(snapshot, history_turn_limit, None)
    }

    fn from_snapshot_with_boundary(
        snapshot: &Thread,
        history_turn_limit: u32,
        boundary: Option<ThreadHistoryBoundary>,
    ) -> Self {
        let oldest_turn_id = boundary
            .as_ref()
            .and_then(|history| history.oldest_turn_id.clone())
            .or_else(|| snapshot.turns.first().map(|turn| turn.turn_id.clone()));
        Self {
            session_id: snapshot.session_id.clone(),
            thread_id: snapshot.thread_id.clone(),
            confirmed_sequence: snapshot.sequence,
            history_turn_limit,
            oldest_turn_id,
            has_older_turns: boundary.is_some_and(|history| history.has_older_turns),
            stream_sequences: BTreeMap::new(),
        }
    }
}

fn validate_snapshot_scope(
    snapshot: &Thread,
    session_id: &SessionId,
    thread_id: &ThreadId,
) -> Result<(), ClientError> {
    if snapshot.session_id == *session_id && snapshot.thread_id == *thread_id {
        return Ok(());
    }
    Err(ClientError::Protocol(format!(
        "thread subscription returned snapshot for {}/{}; expected {session_id}/{thread_id}",
        snapshot.session_id, snapshot.thread_id
    )))
}

fn validate_update_scopes(
    updates: &[ThreadUpdateEnvelope],
    session_id: &SessionId,
    thread_id: &ThreadId,
) -> Result<(), ClientError> {
    if let Some(update) = updates
        .iter()
        .find(|update| update.session_id != *session_id || update.thread_id != *thread_id)
    {
        return Err(ClientError::Protocol(format!(
            "thread subscription returned update for {}/{}; expected {session_id}/{thread_id}",
            update.session_id, update.thread_id
        )));
    }
    Ok(())
}

#[cfg(test)]
#[path = "subscription_tests.rs"]
mod tests;
