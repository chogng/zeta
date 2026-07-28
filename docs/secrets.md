# `zeta-secrets` 架构

> - 物理位置：`zeta-rs/secrets/`
> - Rust crate：`zeta_secrets`
> - 层次：host secret persistence primitive
> - 当前实现：typed key/value、`load/store/delete` port、ephemeral memory backend、unavailable backend
> - Crate 实现、安全义务与测试：[`zeta-rs/secrets/README.md`](../zeta-rs/secrets/README.md)
> - Direct-provider credential：[`model-provider.md`](model-provider.md#6-provider-credential-与-subscription-backend)
> - Interactive login control plane：[`login.md`](login.md)
> - App Server 登录控制面：[`zeta-app-server-api.md`](zeta-app-server-api.md#11-account-与登录)

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
- OS keyring、显式 file、ephemeral 等 backend adapter；
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
  zeta-http-client  ── HTTP/WebSocket/proxy/TLS/pool/transport diagnostics
```

`zeta-api`、`zeta-client` 和 `zeta-http-client` 都不依赖 `zeta-secrets`。它们接收已经构造完成的
请求或已经解析的 sensitive transport value，不查 keychain，也不刷新 token。

## 4. Public API

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
```

这里的 account segment 必须是 opaque ID，不能直接放 email、token 或 workspace name。
ChatGPT/Codex subscription token 不在 Zeta namespace 中：它由 upstream Codex App Server own storage
管理，Zeta 只能保存 redacted account reference。
MCP/Connector 使用自己的 namespace，不能把 Provider key schema 当成通用 credential schema。

## 5. Backend policy

长期 backend：

| Host | 默认 backend | 说明 |
| --- | --- | --- |
| Desktop | OS keyring | macOS Keychain、Windows Credential Manager、Linux Secret Service |
| CLI/TUI interactive | OS keyring | 不可用时必须显式选择 file 或 ephemeral，不能静默降级 |
| CI/exec | ephemeral / injected | secret 由进程环境或调用方注入，不自动落盘 |
| Explicit file | opt-in | 只用于无法使用 OS keyring 的 host |

生产 backend 必须一次实现完整的 `load/store/delete`。不保留旧
`MacosKeychainCredentialStore::read_secret`，因为只读 adapter 会迫使 writer 形成第二套 authority。

显式 file backend 的最低要求：

- parent directory 私有，Unix file mode 至少 `0600`；
- Zeta 专属文件名和 schema version；
- temp file + fsync + atomic replace；
- 并发写入有确定的 serialization；
- delete 清理所有 fallback copy；
- error、backup、fixture 和 telemetry 不包含 secret。

OS keyring backend 不得把 secret 放进 process arguments。只有能使用安全平台 API 或安全 stdin
contract 时才能实现；不能为了“先接上”调用会在进程列表暴露 password 的命令行。

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

## 7. 目标目录

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
    ├── secrets_tests.rs
    └── backend/
        ├── mod.rs
        ├── keyring.rs
        ├── file.rs
        └── backend_tests.rs
```

`backend/` 在实现第一个完整生产 backend 时再创建，不预建空模块。

## 8. 依赖方向

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

## 9. 测试门

- load/store/replace/delete/not-found contract；
- `SecretValue` 的 `Debug` 和 error negative logging；
- invalid/control-character key rejection；
- namespace collision；
- concurrent backend access；
- backend unavailable/access denied 分类；
- OS keyring 的完整 round trip；
- file permission、atomic replace、crash recovery；
- logout/delete 后所有 fallback copy 均不可读取；
- schema、App Server DTO、Thread event 和 rollout 中无 secret-bearing field。

## 10. 固定决策

1. 长期保留独立 `zeta-secrets` crate。
2. 删除 `zeta-credentials`；不建立同义的统一 credential/OAuth authority。
3. Direct-provider credential 属于 `zeta-model-provider`，interactive login 属于 `zeta-login`；
   MCP/Connector 各自拥有登录状态机。
4. secrets 只保存 opaque bytes，不理解 token 或 account。
5. Config 只保存 reference，不保存 secret。
6. API/client/Core 不读取 secret store。
7. 生产 backend 未完整实现前，显式返回 unavailable，不静默使用普通明文文件。
8. Zeta App Server 不读取 `SecretStore`，也不拥有 OAuth state/token；ChatGPT/Codex 的凭据归
   upstream Codex App Server。
