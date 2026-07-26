# `zeta-model-provider-config` 架构与演进方案

> - 物理位置：`zeta-rs/model-provider-config/`
> - Rust crate：`zeta_model_provider_config`
> - 层次：声明配置层
> - 当前状态：基础实现已包含声明式默认 `ApiProfile`；多 profile allow-list 与用户 override 仍待实现
> - Provider runtime：[`model-provider.md`](model-provider.md)
> - API 协议层：[`zeta-api.md`](zeta-api.md)

## 1. 结论

`zeta-model-provider-config` 描述“一个 Provider 可以怎样被配置”。它只包含可序列化、可校验、
可生成 schema 的声明值，不创建 HTTP client，不读取 credential，也不执行模型调用。

这里的“静态”不是 Rust 编译期常量，而是指：

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
- base URL normalization 规则；
- 静态 seed models 和 model catalog policy；
- 非敏感调用默认值，例如最大输出 token；
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
- `NormalizedModelProviderConfig`；
- `ProviderDefinition`；
- `ApiProfile`（definition 的显式默认 API profile）；
- `ProviderConfigRegistry`；
- `ProviderAdapter`；
- `EndpointPolicy`；
- `ModelCatalogPolicy`；
- built-in Provider definitions；
- 静态校验、registry merge 和 schema tests。

当前需要演进的地方：

| 当前形态 | 问题 | 目标 |
| --- | --- | --- |
| `ProviderAdapter` 同时近似 Provider 名称和 runtime 实现 | 容易与 `zeta-api::Api` 再建一套分派 | 明确它是 runtime adapter identity，API endpoint 由 runtime 选择 |
| `EndpointPolicy` 只描述 base URL | 名称容易被理解为 `/messages` 等协议 endpoint | 改称或文档化为 `BaseUrlPolicy` 语义 |
| definition 目前只有一个 `api_profile` | 无法表达 Google、xAI、Ollama 等多个正式 API profile | 扩展为 typed default/allowed API profile policy |
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
    ChatGptCodex,
    AnthropicMessages,
    GeminiInteractions,
    GeminiGenerateContent,
    OllamaChat,
}
```

`ApiProfileConfig` 是可序列化的配置值，不是 `zeta-api` runtime object。`zeta-model-provider`
负责把它映射为具体 endpoint implementation，配置 crate 不依赖 `zeta-api`。

`ChatGptCodex` 不是“再加一个可填写 base URL 的 OpenAI-compatible profile”。它只能由 built-in
definition 声明为需要已配置 Codex subscription runtime 的 service surface：target、credential 和
backend selection 由 [`zeta-codex-app-server`](codex-app-server.md) 及 upstream Codex 管理，用户配置
不得覆盖为任意 URL，Platform API key 也不得用于该 profile。上游提供的 memories/search/realtime
能力不自动成为 `zeta-api` endpoint；只有通过公开 App Server contract 映射的能力才可接入。

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

## 5. Merge 与 normalization

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

## 6. Base URL 与 endpoint 的边界

三种概念必须分开：

| 概念 | Owner | 示例 |
| --- | --- | --- |
| 默认 base URL | `model-provider-config` | `https://api.anthropic.com` |
| resolved runtime target | `model-provider` | base URL + credential/runtime headers |
| relative API endpoint | `zeta-api` | `POST /v1/messages` |

配置层禁止把完整 invocation URL 当成通用字符串模板，也禁止通过删除 `/v1` 猜测 catalog 或原生
endpoint。Google 和 Ollama 的 invocation/catalog 地址可能不共享同一个 base path，必须由
definition/runtime 显式声明。

## 7. 静态 model metadata

`ProviderDefinition.seed_models` 是：

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
manager resolution，而不是让静态 definition 永久承担动态 catalog。

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
