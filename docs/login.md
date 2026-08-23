# 登录与账户系统

> 物理位置：`zeta-rs/login/`
> Rust crate：`zeta_login`
> 当前状态：控制面、App Server RPC、Codex managed-login driver 与 Codex Turn backend adapter 已实现；
> 产品通过显式选择 `provider = openai, access = subscription` 的模型接入 subscription Turn
> ChatGPT/Codex subscription adapter：[`codex-app-server.md`](codex-app-server.md)
> Provider runtime：[`model-provider.md`](model-provider.md)
> Secret persistence：[`secrets.md`](secrets.md)

## 快速理解

登录系统是面向用户的账户控制面，不是通用 OAuth 实现。当前 `LoginService` 已拥有稳定 login ID、
取消、完成、登出和 revisioned account projection；具体 ChatGPT/Codex 登录由
`zeta-codex-app-server` driver 委托上游 Codex App Server。

| 用户动作或凭据类型 | 由谁处理 | Zeta 登录系统能看到什么 |
| --- | --- | --- |
| ChatGPT/Codex 浏览器或设备码登录 | 上游 Codex App Server | 授权地址、一次性用户码和脱敏账户状态 |
| 登出、取消或切换账户 | 登录控制面协调，供应商适配器执行 | 稳定状态和脱敏结果 |
| OpenAI、Anthropic 等 API key | 对应模型凭据领域 | 不属于交互式登录 |
| AWS 凭据链、Google ADC、Azure 托管身份 | 对应供应商运行时 | 不包装成通用 OAuth |
| access token、refresh token、cookie | 精确供应商适配器或上游服务 | 不进入 Zeta RPC、日志或遥测 |

## 1. 结论

`zeta-login` 是用户可见的身份登录控制面。它统一表达开始、取消、完成、登出、账户切换和
reauthentication-required 状态；它不把不同 Provider 的 credential 协议伪装成一套通用 OAuth
实现。

当前实现是 provider-neutral control plane：`InteractiveLoginDriver` 接收 service-owned `LoginId`，
只返回 browser/device-code UI instruction 和 redacted account snapshot。App Server 已暴露
`account/read`、`account/login/start`、`account/login/cancel`、`account/logout`，并主动发布
`account/login/completed` 与 `account/updated`。本地默认 composition 已安装
**ChatGPT/Codex managed-login driver**：它懒启动上游 `codex app-server`，由上游完成 token 持久化
和刷新；Zeta 不读取、保存、交换或刷新 ChatGPT token。上游 thread/Turn 已由
`CodexTurnExecutionBackend` 映射到 Core。默认 App Server 将订阅模型投影为 `provider = openai`、
`access = subscription`；只有 Session 显式选中该 ModelRef 后，新 Turn 才走订阅后端。登录账户适配器
内部的 `openai-chatgpt` identity 不进入模型目录。
登录完成不会自动切换已有 Session 或 Thread 的执行路径。

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

## 4. 最小公共边界

当前公共 trait 保持 consumer-owned，并要求 driver 保留 service 分配的 exact `LoginId`：

```rust
/// Starts and observes one provider-owned interactive account login.
///
/// Implementations keep provider credentials private. They may return only
/// redacted UI instructions and must make cancellation idempotent for one login.
pub trait InteractiveLoginDriver: Send + Sync {
    fn read_account(&self) -> Result<Option<AccountSnapshot>, LoginError>;
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
zeta-codex-app-server ──▶ zeta-core      # implements TurnExecutionBackend

zeta-login -/-> zeta-secrets
zeta-login -/-> zeta-api / zeta-client / zeta-http-client
zeta-model-provider -/-> zeta-login
```

`zeta-app-server` 是 composition root：它已将 Codex adapter 注入 login service，并把 redacted
control plane 映射到 RPC。Codex Turn adapter 实现 Core 的 `TurnExecutionBackend`，不经过
`zeta-model-provider`；产品层仍需用显式 model/backend selection 决定哪些新 Turn 使用它。用户点击
登录本身不能启动浏览器之外的模型执行，也不能隐式替换执行后端。

## 6. 固定决策

1. `zeta-login` 是登录控制面，不是 credential manager 或 OAuth protocol crate。
2. ChatGPT/Codex subscription 委托给上游 Codex App Server；不直接复用其 OAuth client ID 或
   backend token。
3. 仅在官方允许时增加其他 interactive login adapter；Anthropic subscription 不进入此范围。
4. Secret persistence 仍是各 credential owner 对 `zeta-secrets` 的直接依赖；login service 不读取
   secret bytes。
5. API key 的录入可以由 App Server 的 account/settings command 触发，但它不是 OAuth login，不能
   迫使 `zeta-login` 持有 API key。
