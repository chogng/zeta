use zeta_app_server_client::AppServerClient;
use zeta_app_server_client::ClientError;
use zeta_app_server_client::JsonRpcTransport;
use zeta_app_server_protocol::protocol::session::SessionThreadReadParams;
use zeta_app_server_protocol::protocol::session::SessionThreadSubscribeParams;
use zeta_app_server_protocol::protocol::session::SessionThreadUnsubscribeParams;
use zeta_protocol::SessionId;
use zeta_protocol::Thread;
use zeta_protocol::ThreadId;
use zeta_protocol::ThreadUpdateEnvelope;

/// Tracks the server subscription and last canonical snapshot for the active Thread.
///
/// It does not apply Thread events locally. A newer durable sequence asks the caller to read a
/// fresh authoritative snapshot, which keeps the product reducer in Core.
#[derive(Debug)]
pub(crate) struct ThreadSubscription {
    session_id: SessionId,
    thread_id: ThreadId,
    confirmed_sequence: u64,
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
        })?;
        validate_snapshot_scope(&result.thread, session_id, thread_id)?;
        validate_update_scopes(&result.updates, session_id, thread_id)?;

        let snapshot = if result
            .updates
            .iter()
            .any(|update| update.durable_sequence > result.thread.sequence)
        {
            client
                .read_session_thread(SessionThreadReadParams {
                    session_id: session_id.clone(),
                    thread_id: thread_id.clone(),
                })?
                .thread
        } else {
            result.thread
        };
        validate_snapshot_scope(&snapshot, session_id, thread_id)?;

        Ok((Self::from_snapshot(&snapshot), snapshot))
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
            let snapshot = client
                .read_session_thread(SessionThreadReadParams {
                    session_id: session_id.clone(),
                    thread_id: thread_id.clone(),
                })
                .map(|result| result.thread)?;
            validate_snapshot_scope(&snapshot, session_id, thread_id)?;
            *self = Self::from_snapshot(&snapshot);
            return Ok(ThreadSwitch::Complete { snapshot });
        }

        let previous_thread_id = self.thread_id.clone();
        let (next, snapshot) = Self::start(client, session_id, thread_id)?;
        *self = next;
        let cleanup = client.unsubscribe_session_thread(SessionThreadUnsubscribeParams {
            session_id: self.session_id.clone(),
            thread_id: previous_thread_id,
        });
        match cleanup {
            Ok(()) => Ok(ThreadSwitch::Complete { snapshot }),
            Err(error) => Ok(ThreadSwitch::StaleSubscription { snapshot, error }),
        }
    }

    pub(crate) fn requires_snapshot(&self, update: &ThreadUpdateEnvelope) -> bool {
        update.thread_id == self.thread_id
            && (update.session_id != self.session_id
                || update.durable_sequence > self.confirmed_sequence)
    }

    pub(crate) fn confirm_sequence(&mut self, sequence: u64) {
        self.confirmed_sequence = self.confirmed_sequence.max(sequence);
    }

    fn from_snapshot(snapshot: &Thread) -> Self {
        Self {
            session_id: snapshot.session_id.clone(),
            thread_id: snapshot.thread_id.clone(),
            confirmed_sequence: snapshot.sequence,
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
