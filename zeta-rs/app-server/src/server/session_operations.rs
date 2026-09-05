//! Session creation, discovery, subscriptions, and lifecycle mutations.

use super::AppServer;
use super::ConnectionState;
use super::RpcError;
use super::core_error;
use super::decode;
use super::operations::SessionMutation;
use super::result;
use serde_json::Value;
use std::sync::Arc;
use zeta_app_server_protocol::protocol::session::SessionCreateParams;
use zeta_app_server_protocol::protocol::session::SessionListResult;
use zeta_app_server_protocol::protocol::session::SessionReadParams;
use zeta_app_server_protocol::protocol::session::SessionResult;
use zeta_app_server_protocol::protocol::session::SessionSubscribeParams;
use zeta_app_server_protocol::protocol::session::SessionSubscribeResult;
use zeta_app_server_protocol::protocol::session::SessionThreadProjection;
use zeta_app_server_protocol::protocol::session::SessionUnsubscribeParams;
use zeta_core::StartThreadRequest;
use zeta_protocol::SessionId;

impl AppServer {
    pub(super) fn session_create(
        &self,
        connection: &mut ConnectionState,
        params: &Value,
    ) -> Result<Value, RpcError> {
        let params: SessionCreateParams = decode(params)?;
        let created = self
            .start_thread(StartThreadRequest {
                command_id: params.command_id,
                title: params.title,
            })
            .map_err(core_error)?;
        self.updates.bind_session_scope(created.session_id.clone());
        self.threads
            .install_session_extensions(
                created.session_id.clone(),
                Arc::clone(&self.agent_extensions),
            )
            .map_err(core_error)?;
        self.updates
            .subscribe_session(connection.connection_id, created.session_id.clone());
        result(&SessionResult {
            session: self.session_view(&created.session_id)?,
        })
    }

    pub(super) fn session_read(&self, params: &Value) -> Result<Value, RpcError> {
        let params: SessionReadParams = decode(params)?;
        result(&SessionResult {
            session: self.session_view(&params.session_id)?,
        })
    }

    pub(super) fn session_list(&self) -> Result<Value, RpcError> {
        result(&SessionListResult {
            sessions: self.session_views()?,
        })
    }

    pub(super) fn session_subscribe(
        &self,
        connection: &mut ConnectionState,
        params: &Value,
    ) -> Result<Value, RpcError> {
        let params: SessionSubscribeParams = decode(params)?;
        let session = self.session_view(&params.session_id)?;
        let thread_snapshots = self
            .threads
            .list_session_threads(&params.session_id)
            .map_err(core_error)?;
        let thread_projections = thread_snapshots
            .iter()
            .map(|thread| {
                let thread = thread.public_thread();
                let updates = self
                    .threads
                    .thread_updates_after(&thread.thread_id, 0)
                    .map_err(core_error)?;
                Ok(SessionThreadProjection {
                    transcript: self.updates.thread_transcript_snapshot(&thread, true),
                    thread,
                    updates,
                })
            })
            .collect::<Result<Vec<_>, RpcError>>()?;
        self.updates
            .subscribe_session(connection.connection_id, params.session_id.clone());
        for item in &thread_projections {
            self.updates.subscribe_session_thread(
                connection.connection_id,
                params.session_id.clone(),
                item.thread.thread_id.clone(),
                item.thread.sequence,
            );
        }
        for snapshot in &thread_snapshots {
            self.offer_pending_interactions(snapshot);
        }
        result(&SessionSubscribeResult {
            agent_tree: zeta_core::project_agent_tree(&thread_snapshots),
            session,
            thread_projections,
        })
    }

    pub(super) fn session_unsubscribe(
        &self,
        connection: &mut ConnectionState,
        params: &Value,
    ) -> Result<Value, RpcError> {
        let params: SessionUnsubscribeParams = decode(params)?;
        let lost_dynamic_tools = self
            .updates
            .unsubscribe_session(connection.connection_id, &params.session_id);
        self.cancel_lost_dynamic_tool_owners(lost_dynamic_tools);
        Ok(Value::Null)
    }

    pub(super) fn archive_session_request(
        &self,
        mutation: SessionMutation,
    ) -> Result<SessionResult, RpcError> {
        let session_id = mutation.session_id.clone();
        let result = self.lifecycle_request(mutation)?;
        self.clear_session_dirs(&session_id);
        Ok(result)
    }

    pub(super) fn restore_session_request(
        &self,
        mutation: SessionMutation,
    ) -> Result<SessionResult, RpcError> {
        let restored = self
            .threads
            .restore_session(&mutation.session_id)
            .map_err(core_error)?;
        self.notify_thread_updates(&restored.thread_id, restored.sequence.saturating_sub(1))?;
        self.updates.publish_session_changed(&mutation.session_id);
        Ok(SessionResult {
            session: self.session_view(&mutation.session_id)?,
        })
    }

    pub(super) fn stop_session_request(
        &self,
        mutation: SessionMutation,
    ) -> Result<SessionResult, RpcError> {
        let thread_sequences = self
            .threads
            .list_session_threads(&mutation.session_id)
            .map_err(core_error)?
            .into_iter()
            .map(|thread| (thread.thread_id, thread.sequence))
            .collect::<Vec<_>>();
        self.threads
            .archive_session_threads(
                &mutation.session_id,
                &mutation.command_id,
                zeta_protocol::ThreadArchiveReason::Stopped,
            )
            .map_err(core_error)?;
        self.clear_session_dirs(&mutation.session_id);
        for (thread_id, _) in &thread_sequences {
            self.multi_agent
                .cancel_descendants(thread_id)
                .map_err(core_error)?;
        }
        for (thread_id, sequence) in thread_sequences {
            self.notify_thread_updates(&thread_id, sequence)?;
        }
        self.updates.publish_session_changed(&mutation.session_id);
        self.enforce_turn_changes_cleanup();
        Ok(SessionResult {
            session: self.session_view(&mutation.session_id)?,
        })
    }

    pub(super) fn delete_session_request(
        &self,
        mutation: SessionMutation,
    ) -> Result<SessionId, RpcError> {
        let session_id = mutation.session_id.clone();
        let thread_ids = self
            .threads
            .list_session_threads(&session_id)
            .map_err(core_error)?
            .into_iter()
            .map(|thread| thread.thread_id)
            .collect::<Vec<_>>();
        self.threads
            .archive_session_threads(
                &session_id,
                &mutation.command_id,
                zeta_protocol::ThreadArchiveReason::Stopped,
            )
            .map_err(core_error)?;
        for thread_id in &thread_ids {
            self.multi_agent
                .cancel_descendants(thread_id)
                .map_err(core_error)?;
        }
        self.threads
            .delete_session_threads(&session_id)
            .map_err(core_error)?;
        self.clear_session_dirs(&session_id);
        self.updates.publish_session_deleted(&session_id);
        self.updates.forget_session(&session_id);
        self.enforce_turn_changes_cleanup();
        Ok(session_id)
    }

    fn lifecycle_request(&self, mutation: SessionMutation) -> Result<SessionResult, RpcError> {
        self.threads
            .archive_session_threads(
                &mutation.session_id,
                &mutation.command_id,
                zeta_protocol::ThreadArchiveReason::Completed,
            )
            .map_err(core_error)?;
        self.updates.publish_session_changed(&mutation.session_id);
        self.enforce_turn_changes_cleanup();
        Ok(SessionResult {
            session: self.session_view(&mutation.session_id)?,
        })
    }

    fn enforce_turn_changes_cleanup(&self) {
        if let Some(runtime) = &self.turn_changes
            && let Err(error) = runtime.enforce_cleanup_policy()
        {
            log::warn!("Thread worktree cleanup policy failed: {error}");
        }
    }
}
