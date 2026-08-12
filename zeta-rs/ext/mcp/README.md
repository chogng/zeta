# MCP extension

`zeta-mcp-extension` owns the host-facing MCP runtime integration. It materializes enabled
`zeta-config` declarations, keeps the asynchronous MCP sessions alive behind a synchronous
`zeta-core::ToolService`, projects exact source generations, and produces the matching approval
policy.

The lower `zeta-mcp` crate remains the protocol/session domain. App Server is only the composition
root: it installs the returned tool service into the shared registry and replaces it at config safe
points.

Credential resolution and extension-contributed MCP overlays are not implemented yet. An enabled
declaration containing an unresolved credential reference fails closed before a session starts.
