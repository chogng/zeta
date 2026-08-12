# Connectors extension

> Connector domain contract 由 [`zeta-connectors`](../../connectors/README.md) 维护；跨系统语义见
> [`docs/connectors.md`](../../../docs/connectors.md)。

`zeta-connectors-extension` 将 validated Plugin manifest 中的 `ConnectorContribution` 转换为
`ConnectorDefinition`，保留 Plugin provenance，把 disconnected entry 投影为 catalog-only discovery，
并只为 connected domain entry 输出 ready MCP server ID。

`ConnectorCatalog::from_manifests` 是 Plugin → Connector adapter；
`ConnectorCatalog::with_connection_update` 委托 `ConnectorSnapshot` 校验 generation/state；
`discovery_snapshot` 只产生不可转换为 `ToolDefinition` 的 `Connect` candidate；
`ready_mcp_server_ids` 只投影 binding identity，不启动 session。

本 crate 不拥有 Connector identity/state machine、OAuth、secret values、MCP session 或 tool registry。
前两类 domain contract 属于 `zeta-connectors`，认证 adapter 更新 connection snapshot，实际 MCP runtime
属于 `zeta-mcp-extension`。

验证入口：

```bash
cargo test -p zeta-connectors-extension
cargo clippy -p zeta-connectors-extension --all-targets --no-deps -- -D warnings
```
