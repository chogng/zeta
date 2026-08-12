# Connectors extension

`zeta-connectors-extension` owns the identity and lifecycle boundary for external product accounts.
It projects connector declarations from validated Plugin manifests, keeps non-secret account state
separate from package identity, exposes disconnected entries through capability discovery, and
gates their backing MCP server IDs until an account is connected.

This crate does not perform OAuth, store secret values, or start MCP sessions. The credential owner
updates `ConnectorConnectionState`; `zeta-mcp-extension` owns the resulting MCP runtime.
