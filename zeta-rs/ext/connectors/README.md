# Connectors extension

> Connector domain contract 由 [`zeta-connectors`](../../connectors/README.md) 维护；跨系统语义见
> [`docs/connectors.md`](../../../docs/connectors.md)。

`zeta-connectors-extension` 拥有 Plugin projection、durable connection authority 和 credential
orchestration。它不拥有 Connector state machine、secret backend、MCP session 或 Tool registry；这些分别
属于 `zeta-connectors`、`zeta-secrets` 和 `zeta-mcp-extension`。

## 当前公共契约

| Symbol | 职责 | 关键失败语义 |
| --- | --- | --- |
| `ConnectorCatalog::from_packages` | 用 exact package digest 构造授权兼容 revision | duplicate identity / invalid contribution fail closed |
| `ConnectorCatalog::from_manifests` | 无 package handle 时的较弱 declaration projection | 不覆盖 MCP definition 文件内容；生产 activation 应优先使用 package API |
| `ConnectorAuthority::open_sqlite` | 恢复 snapshot、事件和 command receipts | event + receipt 在一个 SQLite transaction 中提交 |
| `ConnectorAuthority::apply` | expected-generation CAS 与 exact command replay | 同 ID 不同 payload 为 `CommandConflict` |
| `ConnectorCredentialService::connect_api_token` | Begin → secret store → Complete | secret store 失败时不会发布 `Connected` |
| `ConnectorCredentialService::disconnect` | 先撤销 readiness，再 best-effort delete secret | cleanup 失败返回 `RetryRequired`，不回滚断连 |
| `ConnectorAuthority::with_authorized_invocation` | 把 dispatch 与 disconnect commit 线性化 | stale generation/digest 不执行 closure |

## 内部调用路径

```text
ConnectorCredentialService::connect_api_token
  -> phase_command_id
  -> ConnectorAuthority::apply(BeginConnect)
       -> command::command_digest
       -> authority::event_for_request
       -> SqliteAuthority::persist
  -> SecretStore::store
  -> ConnectorAuthority::apply(CompleteConnect)

ConnectorAuthority::open_sqlite
  -> SqliteAuthority::open
  -> load_latest_records + load_receipts
  -> PersistedRecord::restore
  -> ReauthorizationRequired when definition digest changed
```

`authority::event_for_request` 拥有 command 到 durable event 的绑定；`SqliteAuthority::persist` 拥有
event/receipt 原子性；`auth::credential_key` 只生成 hashed non-PII key。若这些 helper 开始读取 token bytes、
启动 MCP，或 App Server 直接写 SQLite，即表示 ownership 漂移。

当前仅实现 API-token adapter。OAuth browser/device flow、refresh、远端 revoke、生产 secret backend 和
Plugin activation 自动重建 definitions 仍由上层后续实现；不能通过扩张 authority event payload 来保存
OAuth code、refresh token 或 raw credential。

验证入口：

```bash
cargo test -p zeta-connectors-extension
cargo clippy -p zeta-connectors-extension --all-targets --no-deps -- -D warnings
```
