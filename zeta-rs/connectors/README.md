# `zeta-connectors`

> 本 README 是 Connector identity、definition、connection state machine 与 immutable snapshot
> 的当前实现契约。跨 Plugin、认证、MCP 和产品 UI 的系统语义由
> [`docs/connectors.md`](../../docs/connectors.md) 维护。

`zeta-connectors` 是后端无关的 Connector 领域层。它定义外部产品连接是什么、连接状态如何合法
迁移，以及何时可以发布 runtime binding；它不发现 Plugin、不执行 OAuth、不保存 credential bytes，
也不启动 MCP session。

## 公共契约

| Symbol | 当前职责 | 不承担 |
| --- | --- | --- |
| `ConnectorId` | 一个 connectable product declaration 的稳定 identity | Plugin identity、外部账号 identity |
| `ConnectorDefinition` | display metadata 与 runtime-free binding | OAuth 配置、live transport |
| `ConnectorRuntimeBinding` | 描述连接成功后可 materialize 的 runtime；当前为 MCP server ID | MCP session、tool registry |
| `ConnectorAccount` | 外部产品账号与 non-secret credential reference | Zeta login、secret bytes |
| `ConnectorConnection` | connection generation 与状态转换校验 | durable persistence、retry scheduling |
| `ConnectorSnapshot` | generation-bound definitions + connection state | Plugin provenance、产品 discovery DTO |

所有 identity 和展示文本都拒绝空值、控制字符、首尾空白与超长输入。`ConnectorCredentialRef` 只是
authentication adapter 可解释的引用，不能包含或暗示 credential bytes 由本 crate 持有。

## 状态转换与失败语义

一次连接必须按以下顺序发布：

```text
Disconnected / Unavailable / Connected
  -> ConnectorConnectionUpdate::Begin(new connection generation)
  -> Connecting
  -> Connected(account with the exact same connection generation)
     or Unavailable(the exact same connection generation)
```

`Disconnect` 表示 revoke 或显式断开，必须使用更大的 connection generation。每次 update 还必须通过
`ConnectorSnapshot::with_connection_update` 在更大的 snapshot generation 下原子发布。旧 snapshot、
旧 connection attempt、跳过 `Connecting` 的 direct connect 和 unknown Connector 都 fail closed。

`Unavailable` 当前只保留 sanitized reason，不保留上一个 connected account；后续 reconnect 必须开始
新的 connection generation。该约束避免 unavailable state 被误当成仍持有调用 authority。

## 内部所有权与调用路径

```text
ConnectorDefinition::new
  -> definition::validate_text
  -> ConnectorSnapshot::new
       -> sort + duplicate identity rejection

ConnectorSnapshot::with_connection_update
  -> snapshot generation check
  -> ConnectorConnection::apply
       -> connection generation + transition check
  -> publish cloned immutable snapshot
```

| Private symbol | Ownership | 漂移信号 |
| --- | --- | --- |
| `definition::validate_text` | definition/account/reason 的 bounded plain-text invariant | 开始解释 provider 或 Plugin schema |
| `ConnectorConnection::apply` | connection transition 与 generation ordering | 执行 OAuth、读取 secrets、启动 MCP |
| `ConnectorSnapshot::with_connection_update` | immutable atomic projection | 持久化 authority 或 App Server event delivery |

如果本 crate 开始依赖 `zeta-plugins`、`zeta-mcp`、`zeta-secrets`、App Server 或产品 UI，即表示依赖
方向发生漂移。对应适配应进入 `zeta-rs/ext/connectors` 或 authentication/runtime owner。

## 集成义务

调用方必须分别提供：

- declaration source 到 `ConnectorDefinition` 的转换和 provenance；
- OAuth/API-key 等 connect flow 与 credential persistence；
- monotonic snapshot/connection generation authority；
- ready binding 到具体 MCP 或 host adapter runtime 的 materialization；
- App Server protocol、UI、audit 和 durable authority persistence。

`zeta-connectors` 不要求用户登录 Zeta。云端目录、跨设备同步和 managed policy 可以作为上层 source
或 authority adapter，但不能改变本地 Connector domain 的 identity 与 transition contract。

## 验证

```bash
cargo test -p zeta-connectors
cargo clippy -p zeta-connectors --all-targets --no-deps -- -D warnings
bazel test //zeta-rs/connectors:connectors-unit-tests
```

当前测试覆盖 duplicate identity、必须经过 `Connecting`、connection generation 匹配、snapshot stale
update 拒绝，以及 disconnect 后 runtime readiness 撤销。

## 当前限制与扩展点

当前只定义 MCP runtime binding；将来只有在出现真实的非 MCP consumer 后，才增加新的 binding
variant。持久化恢复、OAuth/connect/revoke authority、credential materialization、health/reconnect 和
App Server projection 尚未实现，分别属于上层 integration 与具体 runtime，而不是扩张本 crate 的
I/O ownership。
