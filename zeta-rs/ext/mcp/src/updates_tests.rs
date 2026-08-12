use std::time::Duration;

use zeta_rmcp_client::McpClientEvent;

use super::McpCatalogUpdates;

#[test]
fn tool_list_changes_publish_reconcile_hints_but_other_events_do_not() {
    let updates = McpCatalogUpdates::default();
    let subscription = updates.subscribe();
    let host = updates.client_host();

    host.on_event(McpClientEvent::ResourceListChanged);
    assert!(subscription.try_recv().is_err());

    host.on_event(McpClientEvent::ToolListChanged);
    subscription
        .recv_timeout(Duration::from_secs(1))
        .expect("tool list change must request reconciliation");
}
