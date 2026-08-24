# 模型供应商配置

> - 物理位置：`zeta-rs/model-provider-config/`
> - Rust crate：`zeta_model_provider_config`
> - 层次：声明配置层
> - 当前状态：基础实现已包含声明式默认 `ApiProfile` 和独立 input-token count binding；invocation
>   WebSocket profile 也已使用独立的 fail-closed capability；多 profile allow-list 与用户 override
>   仍待实现
> - Crate 实现与修改路径：[`zeta-rs/model-provider-config/README.md`](../zeta-rs/model-provider-config/README.md)
> - Provider runtime：[`model-provider.md`](model-provider.md)
> - API 协议层：[`zeta-api.md`](zeta-api.md)

## 快速理解

模型供应商配置只描述“允许怎样配置”，并以确定性方式校验和归一化；它不读取凭据、不访问网络，
也不执行模型请求。

| 读者首先会问 | 直接答案 | 深入阅读 |
| --- | --- | --- |
| 这里保存什么？ | 可序列化的供应商定义、API 配置档案、默认值和唯一静态模型目录 | [静态模型元数据](#7-静态模型元数据) |
| 用户覆盖如何生效？ | 按字段规则合并后进行确定性规范化，相同输入必须得到相同结果 | [合并与规范化](#5-合并与规范化) |
| 可以在这里读取 API key 吗？ | 不可以；配置只能保存不敏感的凭据引用 | [拥有与不拥有](#2-拥有与不拥有) |
| 可以探测端点是否可用吗？ | 不可以；网络、凭据和运行时状态不能参与静态配置校验 | [Base URL 边界](#6-base-url-与端点的边界) |
| 当前完成到哪里？ | 已有基础声明、默认 HTTP `ApiProfile` 和独立 WebSocket profile，多档案允许列表与用户覆盖仍待实现 | [当前实现审计](#3-当前实现审计) |

## 1. 结论

`zeta-model-provider-config` 描述“一个 Provider 可以怎样被配置”。它只包含可序列化、可校验、
可生成 schema 的声明值，不创建 HTTP client，不读取 credential，也不执行模型调用。

这里的“静态”主要指校验不依赖运行时状态；其中 built-in model 目录确实由一个 Rust 常量维护：

- 值可以在进程外持久化和传输；
- 校验不依赖网络、credential 或当前进程状态；
- 相同 definition、用户配置和 override 必须得到相同的 normalized config；
- 任何 secret、连接、重试计数或动态 catalog observation 都不能进入该层。

一句话边界：

```text
model-provider-config 负责描述和归一化
model-provider        负责解析并运行
zeta-api              负责协议编解码
zeta-client           负责 API operation retry/framing
zeta-http-client      负责底层网络传输
```

## 2. 拥有与不拥有

### 2.1 拥有

- `ProviderId` 对应的声明式 Provider definition；
- Provider display name 和非敏感静态 metadata；
- 默认 base URL 或“必须由用户配置”的 endpoint policy；
- runtime adapter identity；
- 允许选择的 API profile 及默认 profile；
- exact WebSocket API profile；未声明时必须保持 unavailable；
- input-token count profile、独立 target 与 model eligibility policy；
- base URL normalization 规则；
- 唯一 `STATIC_MODEL_CATALOG`、由它投影的 seed models 和 model catalog policy；
- 非敏感调用默认值，例如最大输出 token；
- 用户按模型 ID 提供的 context window 与 automatic compaction threshold；
- 用户 `ModelProviderConfig`；
- definition、用户配置和 override 的确定性 merge；
- 静态 validation、JSON Schema 和配置错误。

### 2.2 不拥有

- API key、OAuth token、cookie 或签名；
- credential reference 的解析和 refresh；
- resolved runtime header；
- DNS、TCP、TLS、proxy、HTTP client 或连接池；
- retry、deadline、cancellation、SSE 或 telemetry；
- Provider request/response DTO；
- prompt cache 的运行时资源和 usage；
- 动态模型 discovery、TTL、availability 或 catalog snapshot；
- `ModelInvoker`、运行中请求或可变 Provider 状态。

## 3. 当前实现审计

当前 crate 已有：

- `ModelProviderConfig`；
- `ModelContextConfig`；
- `NormalizedModelProviderConfig`；
- `ProviderDefinition`；
- `ApiProfile`（definition 的显式默认 API profile）；
- `WebSocketApiProfile`（与 HTTP compatibility 分离的 exact WebSocket wire profile）；
- `InputTokenCountDefinition` 与 normalized count target/model policy；
- `ProviderConfigRegistry`；
- `ProviderAdapter`；
- `EndpointPolicy`；
- `ModelCatalogPolicy`；
- built-in Provider definitions；
- `STATIC_MODEL_CATALOG` 与 `StaticModelSpec`；
- 静态校验、registry merge 和 schema tests。

当前需要演进的地方：

| 当前形态 | 问题 | 目标 |
| --- | --- | --- |
| `ProviderAdapter` 同时近似 Provider 名称和 runtime 实现 | 容易与 `zeta-api::Api` 再建一套分派 | 明确它是 runtime adapter identity，API endpoint 由 runtime 选择 |
| `EndpointPolicy` 只描述 base URL | 名称容易被理解为 `/messages` 等协议 endpoint | 改称或文档化为 `BaseUrlPolicy` 语义 |
| definition 目前只有一个 `api_profile` | 无法表达 Google、xAI、Ollama 等多个正式 API profile | 扩展为 typed default/allowed API profile policy |
| count binding 已独立声明 profile/target/models | invocation 与 count 可能不共享 base path | 保持 definition 显式，禁止 runtime 剥 URL 或猜 model 前缀 |
| WebSocket profile 已独立声明 | compatible HTTP schema 不足以证明 WebSocket lifecycle | 保持 fail closed；runtime target、codec 与 session client 还要分别验证 |
| 静态 models 同时参与运行时可用性判断 | 容易与 models manager catalog 重复 | 仅作为 seed metadata 和 fallback evidence |

迁移期间可以保留现有类型名，但新代码不能继续扩大这些歧义。

## 4. 目标配置模型

以下类型表达目标语义，名称可在实现时调整：

```rust
pub struct ProviderDefinition {
    pub id: ProviderId,
    pub display_name: String,
    pub runtime_adapter: RuntimeAdapterKind,
    pub base_url: BaseUrlPolicy,
    pub api_profiles: ApiProfilePolicy,
    pub websocket_api_profile: WebSocketApiProfile,
    pub input_token_count: Option<InputTokenCountDefinition>,
    pub model_catalog_policy: ModelCatalogPolicy,
    pub seed_models: Vec<Model>,
    pub defaults: ProviderDefaults,
    pub base_url_normalization: BaseUrlNormalization,
}

pub struct ApiProfilePolicy {
    pub default: ApiProfileConfig,
    pub allowed: BTreeSet<ApiProfileConfig>,
}

pub enum ApiProfileConfig {
    OpenAiResponses,
    OpenAiChatCompletions,
    AnthropicMessages,
    GeminiInteractions,
    GeminiGenerateContent,
    OllamaChat,
}
```

`ApiProfileConfig` 是可序列化的配置值，不是 `zeta-api` runtime object。`zeta-model-provider`
负责把它映射为具体 endpoint implementation，配置 crate 不依赖 `zeta-api`。

当前 `WebSocketApiProfile::{Unavailable, OpenAiResponses}` 只表达 exact wire eligibility。OpenAI
definition 声明 `OpenAiResponses`；xAI 虽然上游另有 Responses WebSocket，但当前 definition 仍绑定
Chat Completions，因此保持 `Unavailable`。Generic OpenAI-compatible 也始终默认 unavailable，不能从
HTTP compatibility 推导 WebSocket。

ChatGPT subscription rows 复用 typed `OpenAiResponses` codec，但不复用 Platform target 或 API key。`runtime = chatgpt_subscription` 使 `zeta-model-provider` 从 `zeta-chatgpt` 获取固定 target 与 fresh OAuth headers；用户配置不得覆盖为任意 URL。

Provider-specific compatibility 也必须 typed：

```rust
pub enum ProviderCompatibilityConfig {
    ProviderDefault,
    OpenAiCompatible(OpenAiCompatibilityConfig),
}
```

不能增加如下通用逃生口：

```rust
provider_options: serde_json::Value
```

它会绕过 schema、validation、secret 审计和 profile 配对检查。

## 5. 合并与规范化

输入优先级固定为：

```text
built-in ProviderDefinition
    ↓
registered definition replacement/extension
    ↓
user ModelProviderConfig
    ↓
invocation-independent host override
    ↓
NormalizedModelProviderConfig
```

Normalization 必须：

- 验证 Provider 是否已注册；
- 选择明确的 API profile；
- 验证 profile 在该 definition 的 allowed set 中；
- 解析默认或用户 base URL；
- 按声明规则 trim/保留 trailing slash；
- 验证 HTTP(S) scheme，但不做 DNS 或网络请求；
- 合并非敏感 defaults；
- 保留“未知”和“未配置”的区别；
- 返回不可含 secret 的 `ProviderConfigError`。

Normalization 不得：

- 根据 URL 猜 Provider 或 API profile；
- 读取环境变量或 credential store；
- 请求 `/models` 验证模型；
- 探测 endpoint 是否在线；
- 将配置缺失静默替换为另一个 Provider；
- 根据 model ID 字符串猜 capability。
- 根据 HTTP `ApiProfile` 或 compatible 标签自动开启 WebSocket。

## 6. Base URL 与端点的边界

三种概念必须分开：

| 概念 | Owner | 示例 |
| --- | --- | --- |
| 默认 base URL | `model-provider-config` | `https://api.anthropic.com` |
| resolved runtime target | `model-provider` | base URL + credential/runtime headers |
| relative API endpoint | `zeta-api` | `POST /v1/messages` |

配置层禁止把完整 invocation URL 当成通用字符串模板，也禁止通过删除 `/v1` 猜测 catalog 或原生
endpoint。Google 和 Ollama 的 invocation/catalog 地址可能不共享同一个 base path，必须由
definition/runtime 显式声明。

## 7. 静态模型元数据

`STATIC_MODEL_CATALOG` 是产品内置模型的唯一声明点。每个 `StaticModelSpec` 可以声明 provider/model identity、display name、access kind、execution runtime、context window、automatic compaction threshold、capabilities、reasoning efforts、personality、input-token count eligibility 和 approval-review default。1M 模型用实际 `context_window = 1_000_000` 表示，`has_one_million_context()` 由数值推导，不保存第二个布尔事实。

`ProviderConfigRegistry::builtin()` 把所有 rows 自动注入 `ProviderDefinition.models`、默认审核模型和 count eligibility。Provider 文件只拥有 endpoint、adapter、profile 和 transport 特例，不能再写一份产品模型名单。

`ModelAccess` 当前区分 `ApiKey`、`Subscription`、`Local`、`Enterprise` 和 `Unknown`。`Subscription` 要求客户端使用登录系统中的用户账户，`ApiKey` 要求模型凭据领域中的开发者密钥；二者不能互相降级。它们不能承担 backend routing，`StaticModelRuntime::{ProviderApi, ChatGptSubscription, KimiCode}` 才是独立执行事实；`ModelRef.provider` 只表示模型厂商。认证、订阅权益和远端可用性仍只在实际 Turn 中验证。

这些 rows 是：

- 启动 seed；
- Provider 没有动态 discovery 时的静态来源；
- display name、capability 和 limit 的内置 metadata evidence；
- 离线配置校验的辅助信息。

它不是：

- 当前 credential 可访问模型的实时 authority；
- availability cache；
- pricing/billing authority；
- runtime health probe。

动态发现、缓存、字段级 merge 和筛选属于
[`zeta-models-manager`](models-manager.md)。`ListedOnly` / `AllowUnlisted` 的最终判断应逐步消费
manager resolution，而不是让静态 definition 永久承担动态 catalog。新增 row 会自动进入通用 contract
tests；测试验证 identity 唯一、metadata 自洽、provider/default/count binding 有效，不维护第二份 expected
model 枚举。

## 8. 依赖方向

```text
zeta-protocol
      ▲
      │ shared IDs/model metadata
zeta-model-provider-config
      ▲
      │ normalized declaration
zeta-model-provider
```

允许：

- `zeta-model-provider-config → zeta-protocol`；
- `zeta-model-provider → zeta-model-provider-config`。

禁止：

```text
zeta-model-provider-config → zeta-model-provider
zeta-model-provider-config → zeta-api
zeta-model-provider-config → zeta-client
zeta-model-provider-config → zeta-http-client
zeta-model-provider-config → credentials
zeta-model-provider-config → Core/App Server
```

## 9. 目标目录

```text
zeta-rs/model-provider-config/
├── BUILD.bazel
├── Cargo.toml
├── README.md
└── src/
    ├── lib.rs
    ├── config/
    │   ├── mod.rs
    │   ├── user.rs
    │   ├── normalized.rs
    │   └── config_tests.rs
    ├── definition/
    │   ├── mod.rs
    │   ├── adapter.rs
    │   ├── api_profile.rs
    │   ├── base_url.rs
    │   ├── defaults.rs
    │   └── definition_tests.rs
    ├── registry/
    │   ├── mod.rs
    │   ├── normalize.rs
    │   └── registry_tests.rs
    ├── providers/
    │   ├── mod.rs
    │   ├── openai.rs
    │   ├── anthropic.rs
    │   ├── google.rs
    │   ├── deepseek.rs
    │   └── ...
    └── error.rs
```

目录只在有实现和测试的 vertical slice 中创建。新 test module 使用 sibling
`*_tests.rs`；public trait/type 写明实现和调用约束。

## 10. 验收

- 所有 public 配置值可序列化、反序列化并生成 schema；
- normalization 是确定性的；
- unknown Provider、非法 URL、非法 profile 组合返回 typed error；
- debug/error 不含 secret；
- config crate 没有网络、credential 或 runtime dependency；
- built-in definitions 的 ID、base URL 和默认 profile 有 contract tests；
- 自定义 OpenAI-compatible 配置必须显式选择 compatible profile；
- 未配置行为不会通过 `bool` 或含糊 `Option` 控制。

## 11. 固定决策

1. 本 crate 是声明配置层，不是运行时。
2. “静态”表示无需网络和进程状态，不表示编译期硬编码。
3. 默认 base URL 属于本 crate，resolved target 不属于。
4. API profile 可以被声明，但具体 `zeta-api` endpoint object 由 runtime 选择。
5. Secret、transport、retry、SSE、telemetry 和动态 catalog 永不进入本 crate。
6. Provider-specific option 必须 typed，禁止任意 JSON escape hatch。
