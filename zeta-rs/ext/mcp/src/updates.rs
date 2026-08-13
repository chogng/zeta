use std::sync::Arc;
use std::sync::Mutex;
use std::sync::mpsc;
use std::time::Duration;

use zeta_rmcp_client::McpClientEvent;
use zeta_rmcp_client::McpClientHost;
use zeta_rmcp_client::McpElicitation;

use zeta_core::ToolInteractionService;

tokio::task_local! {
    static ACTIVE_TOOL_INTERACTIONS: Arc<dyn ToolInteractionService>;
}

pub(crate) async fn with_active_tool_interactions<F>(
    interactions: Arc<dyn ToolInteractionService>,
    future: F,
) -> F::Output
where
    F: std::future::Future,
{
    ACTIVE_TOOL_INTERACTIONS.scope(interactions, future).await
}

/// Process-local publication hub for MCP tool-catalog invalidations.
///
/// A host shares one hub across replacement runtimes and subscribes before starting the initial
/// generation. Tool-list notifications only request a full reconcile; they never mutate a live
/// catalog in place.
#[derive(Clone, Default)]
pub struct McpCatalogUpdates {
    subscribers: Arc<Mutex<Vec<mpsc::Sender<()>>>>,
}

impl McpCatalogUpdates {
    /// Subscribes to future tool-catalog invalidations.
    pub fn subscribe(&self) -> McpCatalogUpdateSubscription {
        let (sender, receiver) = mpsc::channel();
        self.subscribers
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push(sender);
        McpCatalogUpdateSubscription { receiver }
    }

    pub(crate) fn client_host(&self) -> Arc<dyn McpClientHost> {
        Arc::new(McpCatalogUpdateHost {
            updates: self.clone(),
        })
    }

    fn publish(&self) {
        self.subscribers
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .retain(|subscriber| subscriber.send(()).is_ok());
    }
}

/// Blocking receiver for MCP tool-catalog invalidations.
pub struct McpCatalogUpdateSubscription {
    receiver: mpsc::Receiver<()>,
}

impl McpCatalogUpdateSubscription {
    /// Receives one pending invalidation without blocking.
    pub fn try_recv(&self) -> Result<(), mpsc::TryRecvError> {
        self.receiver.try_recv()
    }

    /// Waits up to `timeout` for one invalidation.
    pub fn recv_timeout(&self, timeout: Duration) -> Result<(), mpsc::RecvTimeoutError> {
        self.receiver.recv_timeout(timeout)
    }
}

struct McpCatalogUpdateHost {
    updates: McpCatalogUpdates,
}

impl McpClientHost for McpCatalogUpdateHost {
    fn on_event(&self, event: McpClientEvent) {
        if matches!(event, McpClientEvent::ToolListChanged) {
            self.updates.publish();
        }
    }

    fn handle_elicitation(
        &self,
        request: McpElicitation,
    ) -> zeta_rmcp_client::HostFuture<
        Result<zeta_rmcp_client::ElicitResult, zeta_rmcp_client::RmcpErrorData>,
    > {
        let interactions = ACTIVE_TOOL_INTERACTIONS.try_with(Arc::clone).ok();
        crate::elicitation::handle_elicitation(interactions, request)
    }
}

#[cfg(test)]
#[path = "updates_tests.rs"]
mod tests;
