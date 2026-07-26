# `zeta-model-provider` 架构与演进方案

> - 物理位置：`zeta-rs/model-provider/`
> - Rust crate：`zeta_model_provider`
> - 层次：Provider 运行时与选择层
> - 当前状态：同步 unary runtime 已直接组合 `zeta-api` endpoint profile 与当前
>   `zeta-client` 临时 transport；credential、完整 retry execution 和 streaming 尚未实现
> - 声明配置层：[`model-provider-config.md`](model-provider-config.md)
> - API 协议层：[`zeta-api.md`](zeta-api.md)
> - Operation client：[`zeta-client.md`](zeta-client.md)
> - 底层网络：[`zeta-http-client` README](../zeta-rs/http-client/README.md)
> - Secret persistence：[`secrets.md`](secrets.md)
> - Interactive login control plane：[`login.md`](login.md)
> - ChatGPT/Codex subscription runtime：[`codex-app-server.md`](codex-app-server.md)

## 1. 结论

`zeta-model-provider` 把声明配置解析为一次不可变、可运行的模型绑定。它回答：

- 本次调用选择哪个 Provider 和模型；
- 使用哪个 API profile；
- 使用哪个 resolved base URL、deployment 和 credential；
- 哪些 runtime headers、invocation defaults 和 Provider policy 生效；
- 如何把 `zeta-api` 的协议 endpoint、`zeta-client` operation policy 与共享 transport 组合起来。

它不再实现第二套 wire codec，也不拥有底层 HTTP transport。边界可以概括为：

```text
model-provider-config  描述 Provider
model-provider         选择并组合 Provider runtime
zeta-api               编解码 endpoint/request/event
zeta-client            operation retry、framing、telemetry
zeta-http-client       proxy/TLS/HTTP/WebSocket execution
```

## 2. 拥有与不拥有

### 2.1 拥有

- Provider runtime registry 和 adapter selection；
- normalized config 到 immutable runtime 的解析；
- `(ProviderId, ModelId)` resolution；
- API profile selection；
- API-key、cloud identity 等 direct-provider credential reference 解析和 runtime materialization；
- resolved base URL、deployment、account/region scope；
- credential、tenant 和 deployment headers；
- Provider-level invocation defaults；
- retry policy 的语义选择；
- `ModelInvoker` 生命周期；
- `zeta-api` endpoint、`zeta-client` operation 与共享 HTTP client 的组合；
- 对已注入 subscription backend 的 provider/model binding；
- Provider/model/target identity 到安全 runtime error 的补充；
- models manager 所需 `ModelCatalogSource` runtime adapter。

### 2.2 不拥有

- 可持久化配置 schema 和 built-in definition authority；
- Provider request/response/SSE DTO；
- `/responses`、`/messages` 等 relative path；
- JSON、SSE event 或 NDJSON object 的协议解释；
- DNS/TCP/TLS/proxy/connection pool；
- retry attempt loop、backoff sleep 和 `Retry-After` 等待；
- SSE framing、byte buffering、idle timer；
- HTTP client telemetry backend；
- catalog refresh、TTL、merge、filter 和 snapshot；
- Agent loop、tool loop、Thread/Turn durable state。
- 浏览器登录、OAuth callback、token refresh/revoke 或上游 credential persistence。

## 3. 当前实现审计

当前实现已经具备正确的基本方向：

- `ModelProviderRuntime` 拥有配置 registry 和 transport；
- `Provider` 保存 normalized config、definition 和 adapter；
- `ModelInvoker` 表达不可变 Provider/model selection；
- `src/providers/` 按外部服务组织 runtime adapter；
- `NormalizedModelProviderConfig::api_profile` 显式选择 `ApiEndpoint`；
- `model-provider → zeta-api`、`zeta-client` 和 `zeta-http-client`，没有反向依赖。

已移除的重复分派为：

```text
model-provider-config::ProviderAdapter
        ↓
model-provider::providers::instantiate
        ↓
zeta-api::Api::{OpenAi, DeepSeek, Google, ...}
```

`zeta-api::Api` 曾复制了一次 Provider registry。现在 `zeta-api` 仅公开 endpoint profile；
`model-provider::providers/` 保留唯一的 Provider runtime 选择，并把声明式 `ApiProfile` 映射为
`ApiEndpoint`。

`HttpClient` 和 `UreqHttpClient` 当前暂时定义在 `zeta-client`。目标是把 raw transport port 迁入
`zeta-http-client`，runtime 持有共享 client；API codec 构造 opaque byte request，
`zeta-client` 组合 retry/framing，底层保留 status/header/body transport evidence。

## 4. 运行时解析流程

```text
ModelRuntimeRequest
  ├─ ModelRef
  ├─ ModelProviderConfig
  └─ invocation policy
        │
        ▼
ProviderConfigRegistry::normalize
        │
        ▼
Provider runtime adapter
  ├─ resolve credential
  ├─ resolve target/deployment
  ├─ choose API profile
  ├─ choose retry policy
  └─ bind zeta-api endpoint + zeta-client
        │
        ▼
Arc<dyn ModelInvoker>
```

解析结果必须不可变。配置或 credential revision 变化后创建新的 runtime，不在飞行中的调用对象
上做可变热更新。

## 5. Provider 与 API endpoint 的联动

Provider 名称不能等同于 API 协议。同一 Provider 可以选择多个正式 endpoint：

| Provider | 可选 API profile 示例 |
| --- | --- |
| OpenAI Platform | Responses |
| ChatGPT/Codex subscription | upstream Codex App Server 的 thread/turn 与公开 capability（独立 runtime） |
| xAI | Responses、Chat Completions |
| Google | Gemini Interactions、GenerateContent、OpenAI-compatible Chat |
| MiniMax | Anthropic Messages、OpenAI-compatible Chat |
| Ollama | native Chat NDJSON、OpenAI-compatible Chat |
| DeepSeek | OpenAI Chat、Anthropic-compatible endpoint |

目标调用形态：

```rust
let binding = ApiBinding::OpenAiChat {
    endpoint: zeta_api::endpoint::OpenAiChat::deepseek(),
    target: resolved_target,
};
```

以上只是语义示例。关键约束：

- Provider module 选择 endpoint/profile；
- `zeta-api` 实现 request/response/event codec；
- `zeta-client` 组织 operation attempt，并通过 `zeta-http-client` 执行；
- runtime 不根据 URL 或 model ID 猜 profile；
- profile 变化必须是显式配置或 built-in definition 变化；
- 已有配置不能静默迁移到另一正式 API。

### 5.1 OpenAI service surface 不是 OpenAI-compatible profile

公开 OpenAI Platform API 与 ChatGPT/Codex service 都可能使用 `responses`、`models` 或 realtime
这样的相对 path，但它们的 base URL、credential、可用 operation 和 wire 差异不能由 path 推断。
上游 Codex 的 memories/search 等能力是 subscription runtime capability，不是 `zeta-api` endpoint；
详见 [`zeta-api.md`](zeta-api.md#45-openai-platform-与-chatgptcodex-endpoint-清单)。

因此 runtime binding 至少包含如下事实：

```rust
pub enum OpenAiExecutionBinding {
    PlatformApi {
        endpoint: zeta_api::ApiEndpoint,
        target: ResolvedApiTarget,
        credential_scope: CredentialScope,
    },
    CodexSubscription {
        backend: Arc<dyn SubscriptionModelBackend>,
    },
}
```

`CodexSubscription` 不是 `OpenAiCompatibleAdapter` 的用户自定义 base URL 选项。Platform API key、
custom-compatible credential 与 ChatGPT/Codex subscription runtime 彼此不能复用或降级转换。

ChatGPT/Codex subscription 不构造 `ResolvedApiTarget + Bearer token` binding。它只能由已认证的
[`zeta-codex-app-server`](codex-app-server.md) `SubscriptionModelBackend` 构造；上游 Codex 选择
allow-listed target、管理 credential refresh，并执行实际 backend request。

## 6. Provider credential 与 subscription backend

Provider 的共同点只到“调用前需要可用身份”为止。API key、AWS credential chain、Google ADC、
Microsoft identity 和签名请求仍由本 crate 的 direct-provider runtime materialize；它们的 secret
bytes 可以保存在 [`zeta-secrets`](secrets.md)，但浏览器 login 不属于本 crate。

ChatGPT/Codex subscription 是独立 runtime：

```text
zeta-app-server → zeta-login → zeta-codex-app-server → upstream Codex App Server
                                              │
                                              └─ implements SubscriptionModelBackend
                                                            ▲
                                                            │
                                                zeta-model-provider selects it
```

`SubscriptionModelBackend` 是 `zeta-model-provider` 消费的窄 port。它接收已选择的 model/invocation
intent，返回 canonical execution result 和 redacted account/reauthentication outcome；它不接受
raw access token、header map 或 arbitrary ChatGPT base URL。Codex adapter 是第一个实现，但该 port
不以 Codex DTO 为 public API。

401 recovery 也按身份所有者处理：direct-provider credential 可由其 provider runtime 做一次受限
refresh/rebuild；Codex subscription 由 upstream Codex 自己刷新，Zeta 只接收 reauthentication-required
或稳定 execution error。`zeta-client` 不读取 secrets，也不自行刷新或重试认证。

## 7. Header 和 target

| 内容 | Owner |
| --- | --- |
| Platform/default provider base URL | `model-provider-config` |
| 用户 base URL override | 仅 Platform/custom-compatible provider；由 `model-provider-config` 声明、runtime 解析 |
| ChatGPT/Codex service target | upstream Codex App Server；不接受 Zeta generic user override |
| resolved absolute target | `model-provider` |
| API key/cloud identity/tenant header | `model-provider` + direct-provider credential layer |
| relative path、method、content type | `zeta-api::endpoint` |
| API version、协议 beta header | `zeta-api::endpoint/requests` |
| trace propagation、user agent | `zeta-http-client` |
| operation attempt header | `zeta-client` |

Runtime 合并 headers 时必须使用 typed origin 和冲突规则。认证 header 不得被协议层覆盖，协议
必需 header 不得被用户任意删除；所有 secret header 的 `Debug` 必须脱敏。

## 8. Retry 分工

`zeta-client` 拥有 retry 机制，但 runtime 选择 retry policy：

```text
model-provider
  选择 Never / SafeRead / ExplicitIdempotency 等策略
        ↓
zeta-client
  执行 attempt loop、backoff、jitter、Retry-After 和 telemetry
        ↓
zeta-api
  提供 provider error/status 的事实分类
```

Inference 通常是 POST，不能仅因“尚未收到 token”就假定安全重试。默认规则：

- 没有 Provider 明确 idempotency contract 时，inference 使用 `Never`；
- catalog GET 可以使用有上限的 `SafeRead`；
- 已产生 semantic output 后禁止透明 retry；
- direct-provider credential error、validation error 不重试；
- Codex subscription refresh 和 upstream process outcome 由 `zeta-codex-app-server` 与 upstream
  Codex contract 处理，不能套用 HTTP retry；
- fallback model/provider 是 Agent policy，不是 client retry；
- runtime 只选择 typed policy，不自己 sleep 或写 attempt loop。

## 9. Streaming 分工

```text
zeta-client (direct-provider path)
  从 zeta-http-client 消费 HTTP byte stream
  → SSE/NDJSON framing
  → idle deadline / cancellation / backpressure
        ↓
zeta-api::sse
  provider event decode
  → ping/comment/terminal interpretation
  → canonical ModelStreamEvent
        ↓
model-provider
  attach provider/model identity
  → ModelInvoker stream
```

DeepSeek `: keep-alive` 的 frame 边界由 client 识别，作为无 payload 的 SSE comment 交给协议层；
协议层确认它不形成 model output。Anthropic `event: ping` 同样由协议层过滤。Runtime 不解析
`data:` 行或 provider event JSON。

## 10. Catalog source

`zeta-model-provider` 是 models manager 与实际 Provider network runtime 的连接点：

```text
models-manager::ModelCatalogSource
        ▲ implemented by
model-provider
  ├─ resolved catalog target
  ├─ credential scope
  ├─ zeta-api catalog request codec
  └─ zeta-client operation → zeta-http-client HTTP execution
```

Runtime 负责 scope identity、credential revision 和 catalog API binding；manager 负责何时刷新、
缓存、merge 和发布 snapshot。Runtime 不维护第二份 catalog cache。

Codex subscription 的模型/额度 observation 只能由 `zeta-codex-app-server` 在上游公开 account 或
model operation 支持的范围内提供；它不使 models manager 直读或猜测 ChatGPT backend catalog。

## 11. 依赖方向

箭头表示“依赖”：

```text
zeta-model-provider
  ├──▶ zeta-model-provider-config
  ├──▶ zeta-api ───▶ zeta-client
  ├──▶ zeta-client
  ├──▶ zeta-http-client
  ├──▶ zeta-secrets
  └──▶ zeta-protocol

zeta-codex-app-server
  ├──▶ zeta-login
  ├──▶ zeta-model-provider       # implements SubscriptionModelBackend
  └──▶ zeta-protocol

App Server composition
  ├──▶ zeta-model-provider
  ├──▶ zeta-login
  └──▶ zeta-codex-app-server
```

更准确地说：

- `zeta-client` 不依赖 Provider、API codec、Core 或 config；
- `zeta-http-client` 不依赖 Provider、API codec、Core、config 或 secret store；
- `zeta-api` 可依赖 `zeta-client` 的 operation/SSE value；
- `zeta-model-provider` 依赖 config、API、operation client、HTTP client、secrets 和 protocol；
  它定义 `SubscriptionModelBackend` consumer port，但不依赖具体 Codex adapter；
- config 不反向依赖 runtime；
- API/client 都不反向依赖 model-provider。
- `zeta-codex-app-server` 实现 interactive login 与 subscription-model port；
- Zeta App Server 只组合和映射 redacted control-plane DTO，不实现 OAuth host；
- model-provider 不依赖 App Server、Desktop、CLI 或 TUI。

## 12. 目标目录

```text
zeta-rs/model-provider/
├── BUILD.bazel
├── Cargo.toml
├── README.md
└── src/
    ├── lib.rs
    ├── runtime/
    │   ├── mod.rs
    │   ├── provider.rs
    │   ├── model_invoker.rs
    │   ├── resolution.rs
    │   └── runtime_tests.rs
    ├── binding/
    │   ├── mod.rs
    │   ├── api.rs
    │   ├── target.rs
    │   ├── headers.rs
    │   ├── retry.rs
    │   └── binding_tests.rs
    ├── credential/
    │   ├── mod.rs
    │   ├── api_key.rs
    │   ├── materialized.rs
    │   ├── recovery.rs
    │   ├── cloud/
    │   └── credential_tests.rs
    ├── subscription/
    │   ├── mod.rs             # consumer-owned SubscriptionModelBackend port
    │   └── subscription_tests.rs
    ├── providers/
    │   ├── mod.rs
    │   ├── openai.rs
    │   ├── anthropic.rs
    │   ├── google.rs
    │   ├── deepseek.rs
    │   ├── ollama.rs
    │   └── ...
    ├── catalog/
    │   ├── mod.rs
    │   ├── source.rs
    │   └── catalog_tests.rs
    └── error.rs
```

Provider module 只保留确实属于 runtime selection 的差异。Request DTO、SSE event struct 和 HTTP
client implementation 不得放入 `providers/`。

## 13. Public API

Public API 只暴露：

- `ModelProvider`；
- `ModelProviderRuntime` 或更窄的 runtime factory；
- immutable `ModelInvoker`；
- `ModelRuntimeRequest`；
- `SubscriptionModelBackend` consumer port；
- 安全的 `ModelProviderError`；
- models manager 所需的 source adapter。

Provider adapter trait 默认 crate-private。新 public trait 必须写 doc comment，说明配置 snapshot、
credential、cancellation、retry 和 secret redaction 责任。

Public API 不导出 ChatGPT token、PKCE verifier、通用 secret header map 或 `SecretStore`。登录账户
projection 由 [`zeta-login`](login.md) 提供；App Server 只读取该 redacted projection。

## 14. 迁移顺序

1. 建立 `zeta-http-client` 的 unary request/response/config port；
2. 将当前 `zeta-client::UreqHttpClient` 和 raw transport value 迁入底层 crate；
3. 让 `zeta-client` 通过共享 transport 执行 operation retry/framing；
4. 将 `zeta-api::Api` 改为 endpoint/profile API；
5. `model-provider/src/providers/*` 直接选择 endpoint/profile；
6. 增加 streaming binding；
7. 以 OpenAI API key 建立第一个 direct-provider credential vertical slice；
8. 建立 `zeta-login` 与 `zeta-codex-app-server`，以 upstream managed ChatGPT login 接入第一个
   `SubscriptionModelBackend` vertical slice；
9. 实现 models manager 的 catalog source；
10. 删除旧 Provider 级双重 dispatch。

迁移期间不创建空模块，也不同时保留两套长期 public facade。

## 15. 固定决策

1. Provider registry 只存在于 `zeta-model-provider`。
2. Runtime 选择 API endpoint，但不实现 wire codec。
3. Runtime 选择 retry policy，但 retry attempt loop 属于 `zeta-client`。
4. Direct-provider runtime 解析自己的 credential；secret 不进入 config、普通 inference DTO 或
   telemetry。ChatGPT/Codex subscription credential 始终留在 upstream Codex。
5. Runtime 不解析 SSE/NDJSON framing。
6. 动态 catalog cache 属于 models manager。
7. `zeta-api` 和 `zeta-client` 不反向依赖 runtime。
8. direct-provider credential lifecycle 属于本 crate；`zeta-secrets` 只持久化 opaque secret。
9. interactive login lifecycle 属于 `zeta-login`；ChatGPT subscription 由
   `zeta-codex-app-server` 委托给 upstream Codex。
10. 不建立统一 Provider OAuth，也不接入第三方 subscription token。
11. `model-provider` 不创建 ChatGPT backend client；它只消费 injected subscription backend。
