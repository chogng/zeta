# `zeta-login` 架构

> 计划物理位置：`zeta-rs/login/`
> Rust crate：`zeta_login`
> 当前状态：Proposed，尚未创建 crate
> ChatGPT/Codex subscription adapter：[`codex-app-server.md`](codex-app-server.md)
> Provider runtime：[`model-provider.md`](model-provider.md)
> Secret persistence：[`secrets.md`](secrets.md)

## 1. 结论

`zeta-login` 是用户可见的身份登录控制面。它统一表达开始、取消、完成、登出、账户切换和
reauthentication-required 状态；它不把不同 Provider 的 credential 协议伪装成一套通用 OAuth
实现。

第一个实现是 **ChatGPT/Codex subscription**：`zeta-login` 把登录请求交给
`zeta-codex-app-server`，由上游 Codex App Server 完成浏览器/设备码登录、token 持久化和刷新。
Zeta 不读取、保存、交换或刷新 ChatGPT token。

未来只有在 Provider 的官方条款和技术接口明确允许时，才增加新的登录 adapter。API key、AWS
credential chain、Google ADC 和 Azure managed identity 不是 interactive login 的变体，仍由各自
Provider runtime 解析和 materialize。

## 2. 所有权

`zeta-login` 拥有：

- redacted account/session projection；
- login request 的生命周期和稳定 `LoginId`；
- begin/cancel/logout/account-changed/reauthentication-required 的状态转换；
- UI/CLI/App Server 所需的授权 URL、device code、进度和稳定错误的安全投影；
- 对 provider-specific interactive login adapter 的最小 consumer-owned port。

`zeta-login` 不拥有：

- OAuth authorize/token/revoke HTTP codec；
- PKCE、state、callback listener、refresh token 或 cookie；
- API key、AWS SigV4、Google ADC 等非交互凭据的 materialization；
- `SecretStore`、keychain 或 token 的序列化；
- 模型 endpoint、模型请求、retry、SSE 或 telemetry；
- Provider 选择和模型执行。

## 3. 与 Codex subscription 的关系

```text
Zeta Desktop / CLI / TUI
             │ account/login/*
             ▼
zeta-app-server
             ▼
zeta-login
             │ InteractiveLoginDriver
             ▼
zeta-codex-app-server
             │ JSON-RPC + local child-process lifecycle
             ▼
upstream `codex app-server`
             │ managed browser/device-code login and refresh
             ▼
ChatGPT subscription
```

授权 URL 和一次性 device code 可以回传给 UI；access token、refresh token、PKCE verifier、callback
code、cookie 和上游 credential-file path 都不能进入 Zeta App Server RPC、desktop IPC、日志或
telemetry。

上游 App Server 的 transport authentication（例如本地 WebSocket capability token）与 ChatGPT
subscription credential 是两种完全不同的 secret，二者不能复用。

## 4. 最小 public boundary

以下仅表达目标形态。公共 trait 必须保持小，并由 `zeta-login` 作为消费者拥有：

```rust
/// Starts and observes one provider-owned interactive account login.
///
/// Implementations keep provider credentials private. They may return only
/// redacted UI instructions and must make cancellation idempotent for one login.
pub trait InteractiveLoginDriver: Send + Sync {
    fn begin(&self, request: BeginLoginRequest) -> Result<BeginLogin, LoginError>;
    fn cancel(&self, login_id: &LoginId) -> Result<CancelLoginOutcome, LoginError>;
    fn logout(&self, account: &AccountRef) -> Result<(), LoginError>;
}
```

`BeginLogin` 是 tagged result，例如 `Browser { authorization_url }` 或
`DeviceCode { verification_url, user_code }`，而不是多个可混淆的 `Option` 字段。完成事件只携带
success/failure 与 redacted account snapshot。

不要在第一版预建“万能 OAuth driver”：resource/audience、dynamic client registration、device code、
workspace selection 和 consent semantics 不能由一个宽泛 DTO 正确覆盖。新增 Provider 时先定义其
正式能力和专用 adapter，再决定是否能复用此控制面。

## 5. 依赖方向

```text
zeta-app-server ──▶ zeta-login
zeta-codex-app-server ──▶ zeta-login     # implements interactive login driver

zeta-login -/-> zeta-secrets
zeta-login -/-> zeta-api / zeta-client / zeta-http-client
zeta-model-provider -/-> zeta-login
```

`zeta-app-server` 是 composition root：它将 Codex adapter 注入 login service，并把 redacted control
plane 映射到 RPC。`zeta-model-provider` 不因用户点击登录而启动浏览器；它只消费已配置的
`SubscriptionModelBackend`。

## 6. 固定决策

1. `zeta-login` 是登录控制面，不是 credential manager 或 OAuth protocol crate。
2. ChatGPT/Codex subscription 委托给上游 Codex App Server；不直接复用其 OAuth client ID 或
   backend token。
3. 仅在官方允许时增加其他 interactive login adapter；Anthropic subscription 不进入此范围。
4. Secret persistence 仍是各 credential owner 对 `zeta-secrets` 的直接依赖；login service 不读取
   secret bytes。
5. API key 的录入可以由 App Server 的 account/settings command 触发，但它不是 OAuth login，不能
   迫使 `zeta-login` 持有 API key。
