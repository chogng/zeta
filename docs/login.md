# 登录与账户系统

> 物理位置：`zeta-rs/login/`
> Rust crate：`zeta_login`
> 当前状态：多 Provider 控制面、App Server RPC、ChatGPT/Kimi native device OAuth 与 local model runtime 已实现
> ChatGPT 订阅适配器：[`chatgpt-subscription.md`](chatgpt-subscription.md)
> Kimi OAuth owner：`zeta-rs/kimi/`
> Provider runtime：[`model-provider.md`](model-provider.md)
> Secret persistence：[`secrets.md`](secrets.md)

## 快速理解

登录系统是面向用户的多账户控制面，不是通用 OAuth 实现。当前 `LoginService` 按 provider 注册 driver，拥有稳定 login ID、取消、完成、provider-scoped 登出和 revisioned account collection；ChatGPT 与 Kimi 流程分别由 `zeta-chatgpt`、`zeta-kimi` 在本机执行 device authorization、刷新和 SecretStore 持久化。

| 用户动作或凭据类型 | 由谁处理 | Zeta 登录系统能看到什么 |
| --- | --- | --- |
| ChatGPT 设备码登录 | 本机 `zeta-chatgpt` | 授权地址、一次性用户码和脱敏账户状态；token 只进入本机 SecretStore |
| Kimi 设备码登录 | 本机 `zeta-kimi` | 授权地址、一次性用户码和脱敏账户状态；token 只进入本机 SecretStore |
| 登出、取消或切换账户 | 登录控制面协调，供应商适配器执行 | 稳定状态和脱敏结果 |
| 没有受支持订阅 OAuth 的供应商 API key | 对应模型凭据领域 | 不属于交互式登录，也不是 OAuth 失败后的降级 |
| AWS 凭据链、Google ADC、Azure 托管身份 | 对应供应商运行时 | 不包装成通用 OAuth |
| access token、refresh token、cookie | 精确供应商适配器 | 不进入 Zeta RPC、日志或遥测 |

## 1. 结论

`zeta-login` 是用户可见的身份登录控制面。它统一表达开始、取消、完成、登出、账户切换和
reauthentication-required 状态；它不把不同 Provider 的 credential 协议伪装成一套通用 OAuth
实现。

当前实现是 provider-neutral control plane：`InteractiveLoginDriver` 声明自己的 stable provider ID，接收 service-owned `LoginId`，只返回 browser/device-code UI instruction 和 redacted account snapshot。App Server 已暴露 `account/read`、`account/login/start`、`account/login/cancel`、带 provider 参数的 `account/logout`，并主动发布 `account/login/completed` 与 `account/updated`；`account/read` 返回 `accounts[]`，所以 ChatGPT 与 Kimi 可以同时登录。

产品边界按认证能力划分：供应商提供并允许稳定的用户订阅 OAuth 时，通过 `zeta-login` 暴露交互式账户登录；没有该能力但提供开发者 API 的供应商，通过模型凭据领域接受 API key。登录方法中不存在 API key 分支，两条路径也不会自动 fallback。

本地默认 composition 同时安装两个 native driver。`zeta-chatgpt` 与 `zeta-kimi` 各自使用对应 device OAuth endpoint 和 public client ID，在本机交换/刷新 token，并将整个 credential envelope 保存到 profile SecretStore。两者都只向控制面投影脱敏账户。

默认目录中的 `openai/*` subscription rows 显式标记 `runtime = chatgpt_subscription`，`kimi/kimi-k2.7-code` 标记 `runtime = kimi_code`；两者都由本地 `TurnExecutor` 执行，只在 request target 与 credential owner 上不同。`access = subscription` 只是接入方式，不能单独决定 target。现有 API-key rows 保持独立凭据路径，二者不互换。
登录完成不会自动切换已有 Session 或 Thread 的执行路径。

Kimi wire contract 以 [Kimi CLI 的官方 OAuth 实现](https://github.com/MoonshotAI/kimi-cli/blob/main/src/kimi_cli/auth/oauth.py) 为主依据，并与 [CLIProxyAPI 的 Kimi adapter](https://github.com/router-for-me/CLIProxyAPI/blob/main/internal/auth/kimi/kimi.go) 交叉验证。Zeta 请求使用真实 `User-Agent: Zeta/*` 与 `X-Msh-Platform: Zeta`，不伪装成 Kimi CLI 或 CPA。

Desktop 的 Models 设置页已接入这条控制面：ChatGPT 与 Kimi 使用独立账户卡；主进程用系统浏览器打开验证页并把一次性 user code 写入剪贴板，Renderer 只显示 challenge 和脱敏完成状态。一个 provider 暂时不可用时，`LoginService` 仍返回其他 provider 的账户；已有但读取失败的账户会投影为 `Unavailable`。

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
- `SecretStore` backend 或 token 的序列化；
- 模型 endpoint、模型请求、retry、SSE 或 telemetry；
- Provider 选择和模型执行。

## 3. 与 ChatGPT 订阅的关系

```text
Zeta Desktop / CLI / TUI
             │ account/login/*
             ▼
zeta-app-server
             ▼
zeta-login
             │ InteractiveLoginDriver
             ▼
zeta-chatgpt
             │ native device OAuth + SecretStore + refresh
             ▼
ChatGPT subscription
```

授权 URL 和一次性 device code 可以回传给 UI；access token、refresh token、authorization code 和 SecretStore bytes 都不能进入 Zeta App Server RPC、desktop IPC、日志或 telemetry。

## 4. 最小公共边界

当前公共 trait 保持 consumer-owned，并要求 driver 保留 service 分配的 exact `LoginId`：

```rust
/// Starts and observes one provider-owned interactive account login.
///
/// Implementations keep provider credentials private. They may return only
/// redacted UI instructions and must make cancellation idempotent for one login.
pub trait InteractiveLoginDriver: Send + Sync {
    fn provider_id(&self) -> &'static str;
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
zeta-chatgpt ──▶ zeta-login / zeta-secrets / zeta-client
zeta-kimi ──▶ zeta-login / zeta-secrets / zeta-client
zeta-model-provider ──▶ zeta-chatgpt     # consumes fresh ChatGPT API targets
zeta-model-provider ──▶ zeta-kimi        # consumes fresh Kimi API targets

zeta-login -/-> zeta-secrets
zeta-login -/-> zeta-api / zeta-client / zeta-http-client
```

`zeta-app-server` 是 composition root：它把 ChatGPT 与 Kimi 两个 adapter 注入同一个 login service，并让两个 OAuth authority 与 model-provider 共享同一 SecretStore owner。两个 adapter 都只提供 fresh authenticated target，由本地 model-provider/TurnExecutor 执行。用户点击登录不会隐式替换 Session 模型。

## 6. 固定决策

1. `zeta-login` 是登录控制面，不是 credential manager 或 OAuth protocol crate。
2. ChatGPT 订阅使用本机 device OAuth、SecretStore 与固定 Responses target；`zeta-chatgpt` 是 token lifecycle 的唯一 owner。
3. Kimi 订阅使用本机 device OAuth、SecretStore 与 Kimi Coding API；`zeta-kimi` 是 token lifecycle 的唯一 owner。
4. 仅在官方允许且有稳定技术接口时增加其他 interactive login adapter；没有受支持 OAuth 的订阅不能伪装成 API key 登录。
5. Secret persistence 仍是各 credential owner 对 `zeta-secrets` 的直接依赖；login service 不读取
   secret bytes。
6. API key 的录入可以由 App Server 的 settings command 触发，但它不是 OAuth login，不能进入 `LoginMethod`、由 `zeta-login` 持有或作为登录失败后的降级。
