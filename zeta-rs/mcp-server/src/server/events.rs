use super::{JsonRpcId, McpServer, id_value};
use crate::events::{AgentEvents, AgentProgress, InteractionResolution};
use crate::interaction::{decode_elicitation_result, elicitation_params};
use serde_json::{Value, json};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use zeta_protocol::AgentRequestEnvelope;

const MAX_PROGRESS_NOTIFICATIONS: u64 = 256;
const DEFAULT_INTERACTION_TIMEOUT: Duration = Duration::from_secs(10 * 60);

pub(super) struct McpAgentEvents {
    server: McpServer,
    outgoing: mpsc::Sender<String>,
    progress_token: Option<Value>,
    progress_count: AtomicU64,
    last_progress_message: Mutex<Option<String>>,
    cancellation: Arc<AtomicBool>,
}

impl McpAgentEvents {
    pub(super) fn new(
        server: McpServer,
        outgoing: mpsc::Sender<String>,
        progress_token: Option<Value>,
        cancellation: Arc<AtomicBool>,
    ) -> Self {
        Self {
            server,
            outgoing,
            progress_token,
            progress_count: AtomicU64::new(0),
            last_progress_message: Mutex::new(None),
            cancellation,
        }
    }

    fn remove_pending(&self, id: &JsonRpcId) {
        if let Ok(mut pending) = self.server.inner.pending_server_requests.lock() {
            pending.remove(id);
        }
    }
}

impl AgentEvents for McpAgentEvents {
    fn progress(&self, progress: AgentProgress) {
        let Some(progress_token) = &self.progress_token else {
            return;
        };
        let count = self.progress_count.fetch_add(1, Ordering::AcqRel) + 1;
        if count > MAX_PROGRESS_NOTIFICATIONS {
            return;
        }
        if let Ok(mut previous) = self.last_progress_message.lock() {
            if previous.as_deref() == Some(&progress.message) {
                return;
            }
            *previous = Some(progress.message.clone());
        }
        let notification = json!({
            "jsonrpc": "2.0",
            "method": "notifications/progress",
            "params": {
                "progressToken": progress_token,
                "progress": count,
                "message": progress.message
            }
        });
        let _ = self.outgoing.send(notification.to_string());
    }

    fn resolve_interaction(&self, request: &AgentRequestEnvelope) -> InteractionResolution {
        let supports_elicitation = self
            .server
            .inner
            .client_features
            .lock()
            .map(|features| features.elicitation_form)
            .unwrap_or(false);
        if !supports_elicitation {
            return InteractionResolution::Unavailable;
        }
        let Some(params) = elicitation_params(request) else {
            return InteractionResolution::Unavailable;
        };
        let id = JsonRpcId::String(format!(
            "zeta-elicit-{}",
            self.server
                .inner
                .next_server_request
                .fetch_add(1, Ordering::Relaxed)
        ));
        let (response_tx, response_rx) = mpsc::channel();
        if let Ok(mut pending) = self.server.inner.pending_server_requests.lock() {
            pending.insert(id.clone(), response_tx);
        } else {
            return InteractionResolution::Unavailable;
        }
        let outbound = json!({
            "jsonrpc": "2.0",
            "id": id_value(&id),
            "method": "elicitation/create",
            "params": params
        });
        if self.outgoing.send(outbound.to_string()).is_err() {
            self.remove_pending(&id);
            return InteractionResolution::Unavailable;
        }

        let timeout = interaction_timeout(request);
        let started = std::time::Instant::now();
        loop {
            if self.cancellation.load(Ordering::Acquire) || started.elapsed() >= timeout {
                self.remove_pending(&id);
                return InteractionResolution::Unavailable;
            }
            match response_rx.recv_timeout(Duration::from_millis(50)) {
                Ok(Ok(value)) => {
                    return decode_elicitation_result(request, value)
                        .map(InteractionResolution::Respond)
                        .unwrap_or(InteractionResolution::Unavailable);
                }
                Ok(Err(_)) | Err(mpsc::RecvTimeoutError::Disconnected) => {
                    self.remove_pending(&id);
                    return InteractionResolution::Unavailable;
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {}
            }
        }
    }
}

fn interaction_timeout(request: &AgentRequestEnvelope) -> Duration {
    let Some(deadline) = request.interaction.deadline else {
        return DEFAULT_INTERACTION_TIMEOUT;
    };
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;
    Duration::from_millis(deadline.expires_at_unix_ms.saturating_sub(now))
        .min(DEFAULT_INTERACTION_TIMEOUT)
}
