# MCP extension

`zeta-mcp-extension` owns the host-facing MCP runtime integration. It materializes enabled
`zeta-config` declarations, keeps the asynchronous MCP sessions alive behind a synchronous
`zeta-core::ToolService`, projects exact source generations, and produces the matching approval
policy.

The lower `zeta-mcp` crate remains the protocol/session domain. App Server is the composition root:
it installs the returned tool service into the shared registry and replaces it when Config or
Connector authority generations change.

`compose_mcp_tools` handles enabled user Config declarations. `compose_mcp_tools_with_connectors`
also reads the exact ready Connector snapshot, loads each opaque credential through the injected
`SecretStore`, and delegates Plugin-specific transport construction to
`ConnectorMcpRuntimeProvider`. Config/Connector server-ID collisions fail closed.

`PluginConnectorMcpRuntimeProvider::from_activation` is the current package-rooted implementation.
It reads strict bounded JSON from exact installed objects, enforces manifest process/network and
credential-slot ceilings, and materializes both Connector-backed and credential-free standalone
Plugin MCP contributions. `StandaloneMcpServer` is the provider-facing publication contract.

`materialize_connector_servers` binds every resulting server to `ConnectorInvocationFence`.
`McpToolService::review_request` rejects stale connections before approval, while
`McpToolService::execute` calls `ConnectorAuthority::with_authorized_invocation` immediately around
the actual MCP dispatch. This prevents an old prepared call from starting after disconnect commits.
The authority lock may delay disconnect until an already-dispatched call returns; weakening that
linearization requires a replacement contract, not a best-effort state check.

`McpCatalogUpdates` translates runtime `tools/list_changed` notifications into host reconcile
hints. App Server constructs the complete next runtime, then atomically replaces future model safe
points; prepared calls retain their old `ToolService` generation until completion.

Standalone Config credential references remain unsupported and fail before session startup.
OAuth and secret persistence are outside this crate. Connector PKCE orchestration and OS keyring /
explicit-file persistence exist in their owning crates; concrete OAuth providers, browser callback
wiring, refresh, and remote revoke remain unavailable.
