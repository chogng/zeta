# 凭据与秘密系统

> - 物理位置：`zeta-rs/secrets/`
> - Rust crate：`zeta_secrets`
> - 层次：host secret persistence primitive
> - 当前实现：typed key/value、`load/store/delete` port、profile 私有文件、OS keyring、ephemeral memory 与 unavailable backend
> - Crate 实现、安全义务与测试：[`zeta-rs/secrets/README.md`](../zeta-rs/secrets/README.md)
> - OS keyring adapter：[`zeta-rs/keyring-store/README.md`](../zeta-rs/keyring-store/README.md)
> - Direct-provider credential：[`model-provider.md`](model-provider.md#6-供应商凭据边界)
> - Interactive login control plane：[`login.md`](login.md)
> - App Server 登录控制面：[`zeta-app-server-api.md`](zeta-app-server-api.md#11-account-与登录)

## 快速理解

秘密存储只安全保存不透明字节；“这个秘密是什么、何时刷新、谁可以使用”始终由对应业务领域
负责。

| 调用方需求 | 秘密存储负责 | 秘密存储不负责 |
| --- | --- | --- |
| 保存或读取一个敏感值 | 按不透明 `SecretKey` 执行 `load/store/delete` | 解释它是 API key、OAuth token 还是其他凭据 |
| 在不同宿主持久化 | 使用 profile 私有文件或调用方注入的临时 backend | 替调用方选择账户、供应商或授权范围 |
| 后端不可用 | 返回明确错误 | 静默降级到不安全文件或普通配置 |
| 记录诊断信息 | 只记录脱敏错误和不敏感键 | 输出秘密字节、认证标头或完整命令 |
| 删除秘密 | 返回已删除或不存在的明确结果 | 撤销远端 token 或完成供应商登出 |

## 1. 结论

`zeta-secrets` 是长期保留的独立基础设施 crate，但它不是 credential authority，也不是统一 OAuth
框架。它只回答一件事：

> 在不进入普通配置、日志、telemetry 或产品事件的前提下，按 opaque key
> 安全地读取、写入和删除 opaque secret bytes。

单独提取它的原因是 Provider credential、MCP OAuth、Plugin/Connector credential slot 等多个领域
都会需要同一种 host secret persistence；这些领域的登录协议和生命周期却并不相同。共享存储
primitive，不能共享一套虚假的 `CredentialManager`。

## 2. 所有权

### `zeta-secrets` 拥有

- `SecretKey`、`SecretValue` 和 secret-store error；
- `SecretStore::load/store/delete`；
- profile 私有 file、OS keyring、ephemeral 与 unavailable backend adapter；
- Zeta namespace isolation；
- secret value 的 `Debug` 脱敏和内存清理；
- backend access、atomic replacement、权限和 negative logging tests。

### `zeta-secrets` 不拥有

- API key、OAuth token bundle、AWS/Google/Azure identity 的业务类型；
- Provider/account/workspace/tenant/credential revision；
- PKCE、browser callback、device-code、refresh、revoke；
- `Authorization` 或 Provider-specific header；
- endpoint、HTTP retry、SSE/WebSocket、telemetry；
- App Server RPC、Desktop 登录 UI；
- 哪个 Plugin/MCP/provider 可以使用哪个 secret 的 grant policy。

因此 secrets crate 中禁止出现 `OpenAiToken`、`AnthropicCredential`、`McpOAuthSession`、
`AccountSnapshot` 等领域类型。

## 3. 分层

```text
static declaration
  zeta-config / model-provider-config
  └─ 只保存 CredentialRef、account selection、provider auth mode

domain runtime
  model-provider::credential / MCP auth / Connector auth
  ├─ 定义 direct credential/token 类型
  ├─ refresh、revoke、scope 校验
  └─ 把领域 identity 映射为 SecretKey

interactive login
  zeta-login + provider-specific adapter
  └─ 只发布 redacted account 状态；是否使用 SecretStore 由 exact adapter 决定

secret persistence
  zeta-secrets
  └─ load / store / delete opaque bytes

wire and transport
  zeta-api          ── method/path/protocol headers/body/event codec
  zeta-client       ── operation retry/framing/telemetry
  zeta-http-client  ── HTTP + shared proxy/TLS/target policy
  zeta-websocket-client ── WebSocket handshake/message transport
```

`zeta-api`、`zeta-client`、`zeta-http-client` 和 `zeta-websocket-client` 都不依赖 `zeta-secrets`。它们接收已经构造完成的请求或已经解析的 sensitive transport value，不读取 secret backend，也不刷新 token。

## 4. 公共接口

```rust
pub trait SecretStore: Send + Sync {
    fn load(&self, key: &SecretKey) -> Result<Option<SecretValue>, SecretStoreError>;
    fn store(&self, key: &SecretKey, value: &SecretValue) -> Result<(), SecretStoreError>;
    fn delete(&self, key: &SecretKey)
        -> Result<DeleteSecretOutcome, SecretStoreError>;
}
```

- `SecretKey` 可记录，因为它必须是不含 secret/PII 的 opaque identity；
- `SecretValue` 不实现 `Clone`、`Display`、serialization，`Debug` 固定脱敏；
- delete 使用 `DeleteSecretOutcome::{Deleted, NotFound}`，不返回含义模糊的 `bool`；
- backend error 必须先净化，不能携带 secret、raw backend response 或完整 command line；
- store 不枚举所有 secret，避免领域绕过自己的 authority 扫描其他 namespace。

消费领域拥有 key schema。例如 Provider runtime 可以使用：

```text
provider/openai/account/{opaque-account-id}/api-key
provider/openai/account/{opaque-account-id}/platform-service-token
provider/kimi/current/oauth
```

这里的 account segment 必须是 opaque ID，不能直接放 email、token 或 workspace name。
ChatGPT 与 Kimi subscription OAuth 都由本机 provider adapter 持有。`zeta-chatgpt` 将 ID/access/refresh token、expiry、脱敏账户 metadata 与 credential revision 编码成 opaque envelope，整体保存于 `provider/openai-chatgpt/current/oauth`；`zeta-kimi` 使用 `provider/kimi/current/oauth` 保存对应 envelope。只有各自 adapter 可以解释、刷新或删除自己的值。
MCP/Connector 使用自己的 namespace，不能把 Provider key schema 当成通用 credential schema。

## 5. Backend 策略

长期 backend：

| Host | 默认 backend | 说明 |
| --- | --- | --- |
| Desktop | profile 私有文件 | `<profile>/secrets/values/`，hashed 文件名，不调用系统钥匙串 |
| CLI/TUI interactive | profile 私有文件 | 与 Desktop 共用同一 profile authority |
| CI/exec | ephemeral / injected | secret 由进程环境或调用方注入，不自动落盘 |
| Standalone embedded host | injected | host 必须显式注入与自己 authority 一致的 backend |
| Explicit keyring host | OS keyring | host 明确选择时注入 `KeyringSecretStore`，不是 daemon 默认 |

生产 backend 必须一次实现完整的 `load/store/delete`。`LocalProfileRuntime` 为一个 profile 只打开一个 `FileSecretStore`，并把同一个 `Arc<dyn SecretStore>` 注入该 daemon 内所有 App Server connection、Connector、MCP 与 Provider credential adapter。

profile file backend 的最低要求：

- parent directory 私有，Unix file mode 至少 `0600`；
- Zeta 专属文件名和 schema version；
- temp file + fsync + atomic replace；
- 并发写入有确定的 serialization；
- delete 清理 exact value file；
- error、backup、fixture 和 telemetry 不包含 secret。

`config.toml`、SQLite、Marketplace cache 和普通 JSON 只保存 non-secret reference。daemon 默认的 secret bytes 只进入 `<profile>/secrets/values/` 下的 opaque hashed value file；显式 keyring host 只进入 OS credential facility。两者之间不做 fallback、双写或自动迁移。

## 6. 生命周期与一致性

`zeta-secrets` 不提供跨多个 key 的业务事务。领域 runtime 负责：

1. 验证新 credential/token bundle；
2. 编码成有 schema version 的领域 payload；
3. store 到 exact `SecretKey`；
4. 持久化成功后才发布不含 secret 的 account/credential revision；
5. logout/revoke 时，即使远端 revoke 失败也执行本地 delete；
6. credential rotation 后使旧 materialized request snapshot 失效。

同一个 credential 的 single-flight refresh 属于 Provider/MCP/Connector auth manager，不属于
secret backend。Backend 可以序列化物理写入，但不能推断 token expiry 或 account identity。

## 7. 当前实现位置

```text
zeta-rs/secrets/
├── BUILD.bazel
├── Cargo.toml
├── README.md
└── src/
    ├── lib.rs
    ├── error.rs
    ├── value.rs
    ├── store.rs
    ├── memory.rs
    ├── file.rs
    ├── file_windows.rs
    ├── secrets_tests.rs
    └── file_tests.rs
```

```text
zeta-rs/keyring-store/
├── BUILD.bazel
├── Cargo.toml
├── README.md
└── src/
    ├── lib.rs
    └── lib_tests.rs
```

文件 backend 的跨平台实现位于 `zeta-secrets/src/file.rs` 和 `file_windows.rs`。OS keyring adapter 位于独立的 `zeta-keyring-store`，避免基础 value/port crate 强制引入平台 credential 依赖。

## 8. 交互式敏感输入

> 状态：Proposed。持久凭据存储已经实现；模型或工具运行中由用户临时提供的敏感回答尚未形成完整端到端契约。

交互式敏感输入不是 `SecretStore` 的另一个写入口。产生请求的业务领域拥有问题和用途，App Server 把请求发送给准确的 renderer，用户界面使用受保护输入控件收集回答，并把值一次性交给等待中的调用；除非用户明确选择保存为某个领域凭据，否则不得持久化。

该路径必须保证：

- 普通 transcript、Thread Item、rollout、Debug、错误、观测数据和剪贴板历史不包含原值；
- 请求绑定准确的 connection、Thread、Turn 和交互 ID，只能回复一次，取消、超时、窗口关闭和重连都有明确终态；
- 领域调用方只在内存中持有完成当前动作所需的最短时间，使用后清理；
- “敏感”只改变传输、展示和记录规则，不自动授予工具、网络、账户或外部修改权限；
- 只有明确的“保存凭据”流程才能把值交给对应领域的凭据 owner，再由该 owner 写入 `SecretStore`。

ChatGPT 订阅的 `isSecret` 请求是首个明确消费者，完成门见 [`chatgpt-subscription.md`](chatgpt-subscription.md#当前状态与待完成项)。

## 9. 依赖方向

允许：

```text
model-provider direct credential ──▶ zeta-secrets
MCP runtime ─────▶ zeta-secrets
Connector auth ──▶ zeta-secrets
login adapter ───▶ zeta-secrets       # only when the provider owns token persistence
App Server composition ──▶ domain auth services
```

禁止：

```text
zeta-secrets ──▶ model-provider / MCP / Plugin / zeta-api / zeta-client / zeta-http-client / zeta-core
zeta-api ──────▶ zeta-secrets
zeta-client ───▶ zeta-secrets
zeta-http-client ──▶ zeta-secrets
zeta-core ─────▶ zeta-secrets
zeta-config ───▶ secret value
Desktop renderer ──▶ SecretStore
```

## 10. 测试门

- load/store/replace/delete/not-found contract；
- `SecretValue` 的 `Debug` 和 error negative logging；
- invalid/control-character key rejection；
- namespace collision；
- concurrent backend access；
- backend unavailable/access denied 分类；
- OS keyring adapter contract 的完整 round trip；真实平台 keyring round trip 由 opt-in host test 验证；
- file backend 的完整 round trip、permission、atomic replace 与 stale staging cleanup；
- logout/delete 后 exact value file 不可读取；
- schema、App Server DTO、Thread event 和 rollout 中无 secret-bearing field。

## 11. 固定决策

1. 长期保留独立 `zeta-secrets` crate。
2. 删除 `zeta-credentials`；不建立同义的统一 credential/OAuth authority。
3. Direct-provider credential 属于 `zeta-model-provider`，interactive login 属于 `zeta-login`；
   MCP/Connector 各自拥有登录状态机。
4. secrets 只保存 opaque bytes，不理解 token 或 account。
5. Config 只保存 reference，不保存 secret。
6. API/client/Core 不读取 secret store。
7. 本地 composition 默认使用 `<profile>/secrets` 下的私有文件 backend；Unix 强制 0700/0600，Windows 使用 owner-only protected DACL 和 write-through atomic replacement。
8. `zeta-keyring-store` 保留为可注入的平台 adapter；它不是 daemon 默认，不与文件 backend fallback 或双写。
9. `LocalProfileRuntime` 拥有一个 profile 的唯一 `SecretStore`；共享 profile runtime 时注入不同 store 会直接拒绝，不形成第二套 credential authority。
10. App Server protocol 和普通 server operation 不暴露 secret；local composition 只把 `SecretStore` 注入 Connector、MCP、ChatGPT 与 Kimi credential adapter。订阅 token 只在本地 credential owner 与单次模型请求之间流动，不进入 Core Agent Loop 状态。
