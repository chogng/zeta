# 模型调用系统

> - 物理位置：`zeta-rs/model-provider/`
> - Rust crate：`zeta_model_provider`
> - 层次：Provider 运行时与选择层
> - 当前状态：completion runtime 已直接组合 `zeta-api` endpoint profile、`zeta-client` operation
>   retry/stream framing 与 `zeta-http-client` transport；OpenAI Responses、OpenAI-compatible Chat
>   Completions、Anthropic Messages 已使用原生 wire streaming；semantic OpenAI API-key
>   materialization 已有 host-injected `SecretStore` 路径；独立 WebSocket transport 与显式 provider
>   capability 已落地，Responses WebSocket codec、session client 和 runtime binding 尚未接入
> - Crate 实现与 adapter 调用图：[`zeta-rs/model-provider/README.md`](../zeta-rs/model-provider/README.md)
> - 声明配置层：[`model-provider-config.md`](model-provider-config.md)
> - API 协议层：[`zeta-api.md`](zeta-api.md)
> - Operation client：[`zeta-client.md`](zeta-client.md)
> - 底层网络：[`zeta-http-client` README](../zeta-rs/http-client/README.md)
> - WebSocket transport：[`zeta-websocket-client` README](../zeta-rs/websocket-client/README.md)
> - Secret persistence：[`secrets.md`](secrets.md)
> - Interactive login control plane：[`login.md`](login.md)
> - ChatGPT 订阅运行时：[`chatgpt-subscription.md`](chatgpt-subscription.md)

## 快速理解

模型调用系统把用户选择的模型变成一次可执行调用：先确定供应商、模型、服务地址和凭据，再把
请求交给协议、重试和网络层。配置在调用开始时冻结，因此一次调用不会在执行途中悄悄换模型或
凭据。

| 常见问题 | 系统行为 | 深入阅读 |
| --- | --- | --- |
| 本次到底使用哪个模型？ | 根据供应商定义、用户选择和本次调用覆盖生成不可变绑定 | [一次调用如何形成](#1-一次调用如何形成) |
| 自定义服务地址会影响什么？ | 只在供应商配置明确允许时生效，不会把一个服务偷偷当成另一个协议 | [供应商与 API 端点](#5-供应商与-api-端点的联动) |
| 凭据由这里保存吗？ | 不保存；这里只取得本次调用所需的凭据，保存和登录属于相邻系统 | [供应商凭据](#6-供应商凭据边界) |
| 失败后会自动重试吗？ | 只有调用类型明确允许安全重试时才会重试；模型推理默认不能仅凭“没收到输出”重跑 | [重试分工](#8-重试分工) |
| 当前已经能做什么？ | 已具备三种 endpoint 的 unary/HTTP streaming completion、embedding/rerank 和独立 WebSocket transport；模型 WebSocket session 尚未接入 | [当前实现审计](#3-当前实现审计) |

## 1. 一次调用如何形成

```mermaid
flowchart TD
    request["Agent 发起模型调用"] --> select["读取用户选择和调用覆盖"]
    select --> definition["校验供应商定义与模型能力"]
    definition --> identity["解析 direct-provider API key 或云身份"]
    identity --> credential["凭据系统提供本次调用材料"]
    credential --> binding["冻结模型、端点、凭据范围与重试策略"]
    binding --> encode["协议层编码请求"]
    encode --> operation["操作层执行截止时间、重试和流式分帧"]
    operation --> transport["网络层完成 HTTP / WebSocket 传输"]
    transport --> result["返回统一模型结果"]
```

这条流程中，模型调用系统只拥有“选择并冻结本次调用绑定”。协议层解释请求和响应，操作层处理
安全重试与分帧，网络层负责真实连接；任何一层都不能根据 URL 或模型名称重新猜测上层已经作出
的选择。

ChatGPT 与 Kimi 订阅在这里都只是一次模型调用：各自的 OAuth owner 提供已刷新的请求目标，模型调用系统完成协议适配，Zeta Core 继续拥有上下文、工具、批准和循环控制。订阅认证不得创建 Thread、推进 Turn 或实现 `TurnExecutionBackend`。

一次绑定必须明确回答：

- 使用哪个供应商和模型；
- 使用哪个 API 配置档案、服务地址和部署目标；
- 使用哪种凭据来源和作用范围；
- 哪些调用默认值与供应商规则生效；
- 这类操作能否安全重试。

## 2. 谁负责什么

| 责任 | 最终所有者 | 模型调用系统在其中做什么 |
| --- | --- | --- |
| 供应商和模型选择 | 模型调用系统 | 解析选择并生成不可变调用绑定 |
| 合法配置形态 | 模型供应商配置 | 提供定义，模型调用系统只消费 |
| 凭据保存和登录 | 凭据与登录系统 | 请求本次需要的敏感材料，不自行持久化 |
| API 请求和响应含义 | 模型 API 协议层 | 选择明确的协议配置档案 |
| 重试、截止时间和流式分帧 | 操作客户端 | 选择重试策略，不执行重试循环 |
| HTTP、WebSocket、代理和 TLS | 网络层 | 提供解析后的目标和标头 |
| 模型目录刷新 | 模型目录系统 | 提供目录访问适配器，不保存第二份缓存 |

### 2.1 本系统拥有

- 供应商运行时注册与适配器选择；
- 规范化配置到不可变运行时的解析；
- 供应商、模型和 API 配置档案的组合；
- 服务地址、部署、账户与区域作用范围的最终解析；
- 凭据引用到本次调用材料的绑定；
- 供应商级调用默认值与重试策略选择；
- 模型调用器 `ModelInvoker` 的生命周期；
- 协议、操作客户端和共享网络客户端的组合；
- 模型目录系统所需的运行时适配器。

### 2.2 本系统不拥有

- 可持久化配置模式和内置供应商定义；
- 供应商请求、响应和流式事件的数据结构；
- `/responses`、`/messages` 等 relative path；
- JSON、SSE 事件或 NDJSON 对象的协议解释；
- DNS/TCP/TLS/proxy/connection pool；
- 重试循环、退避等待和 `Retry-After` 处理；
- 流式分帧、字节缓冲和空闲计时；
- 模型目录刷新、缓存、合并与筛选；
- Agent 循环、工具循环和 Thread/Turn 持久状态；
- 浏览器登录、OAuth 回调、令牌刷新撤销和上游凭据保存。

## 3. 当前实现审计

当前实现已经具备正确的基本方向：

- `ModelProviderRuntime` 拥有配置 registry 和 lazy transport owner；
- `zeta-chatgpt::ChatGptOAuth` 与 `zeta-kimi::KimiOAuth` 为各自的 subscription row 提供 request-time fresh `ResolvedApiTarget`；
- `Provider` 保存 normalized config、definition 和 adapter；
- `ModelInvoker` 表达不可变 Provider/model selection；
- `src/providers/` 按外部服务组织 runtime adapter；
- `NormalizedModelProviderConfig::api_profile` 显式选择 `ApiEndpoint`；
- `ProviderDefinition::websocket_api_profile` 显式声明 exact WebSocket wire profile，不从 HTTP compatibility 推断；
- `SemanticRuntimeResolver` 将 OpenAI-compatible/OpenAI/Ollama 的 exact config 解析为 immutable embedding/rerank invoker；
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

`HttpClient` 和 `UreqHttpClient` 定义在 `zeta-http-client`；`WebSocketConnector` 和 crate-owned
message/handshake types 定义在 `zeta-websocket-client`。两种 transport 共用
`OutboundNetworkSnapshot`，因此 proxy、TLS/mTLS、connect timeout 和 target filtering 不会分叉。Runtime
当前仍只持有共享 lazy HTTP operation client：
App Server 启动不构造 socket/TLS backend，第一次真实 operation 才 fallibly 创建 transport；API codec
构造 opaque byte request，`zeta-client` 组合 retry/framing，底层保留 status/header/body transport evidence。

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

## 5. 供应商与 API 端点的联动

Provider 名称不能等同于 API 协议。同一 Provider 可以选择多个正式 endpoint：

| Provider | 可选 API profile 示例 |
| --- | --- |
| OpenAI Platform | Responses |
| ChatGPT 订阅 | OpenAI Responses codec + `zeta-chatgpt` native OAuth target |
| Kimi Platform | OpenAI-compatible Chat Completions + API key |
| Kimi Code 订阅 | Kimi Coding OpenAI-compatible Chat Completions + native OAuth target |
| xAI | Responses、Chat Completions |
| Google | Gemini Interactions、GenerateContent、OpenAI-compatible Chat |
| MiniMax | Anthropic Messages、OpenAI-compatible Chat |
| Ollama | native Chat NDJSON、OpenAI-compatible Chat |
| DeepSeek | OpenAI Chat、Anthropic-compatible endpoint |

### 5.1 WebSocket 支持矩阵

核对日期：2026-08-23。这里的“支持 WebSocket”只表示当前文本/Agent 模型调用 API 有明确的官方
WebSocket contract；某个供应商在语音、实时音视频或另一套模型 API 中使用 WebSocket，并不代表
Zeta 当前 adapter 可以切换过去。OAuth 也只决定如何取得 credential，不会自动改变 transport。

| Provider / runtime | 官方公开能力 | 与当前 Zeta invocation profile 的关系 | 当前 Zeta 状态 |
| --- | --- | --- | --- |
| OpenAI Platform | [Beta Responses WebSocket client/server events](https://developers.openai.com/api/reference/cli/resources/beta/subresources/responses) | exact Responses family，但仍是 beta contract | `websocketApiProfile = openAiResponses`；transport 已实现，codec/session/runtime 尚未接入，当前仍走 HTTP/SSE |
| ChatGPT 订阅 | 公开 Platform 文档不能证明 subscription service target 的 WebSocket entitlement/URL | 必须由 `zeta-chatgpt` 的 exact target capability 单独确认，不能只看 provider=`openai` | 尚未启用；不得从 Platform 或本地 Codex 实现静默推导 |
| xAI | [Responses WebSocket mode](https://docs.x.ai/developers/advanced-api-usage/websocket-mode) 明确使用 `wss://api.x.ai/v1/responses` | 上游 exact Responses WS，但 Zeta 当前 xAI definition 仍是 Chat Completions | `Unavailable`；先迁移/验证 Responses adapter，再启用 |
| Google Gemini | [Live API](https://ai.google.dev/api/live) 是 stateful WebSocket | 独立 `BidiGenerateContent`/Live 模型协议，不是当前 OpenAI-compatible Chat route | `Unavailable` |
| Qwen | [文本流式输出](https://www.alibabacloud.com/help/en/model-studio/stream) 使用 SSE；[Realtime API](https://www.alibabacloud.com/help/en/model-studio/realtime) 另有 WebSocket | Realtime 属于 Omni/audio/ASR/TTS 等独立协议 | `Unavailable` |
| MiniMax | [API overview](https://platform.minimax.io/docs/api-reference/api-overview) 的 WebSocket 面向 T2A；文本调用为独立 Chat API | 语音 WebSocket 不能替代当前 text Chat route | `Unavailable` |
| Anthropic | [Messages streaming](https://platform.claude.com/docs/en/build-with-claude/streaming) 使用 SSE | 当前 Messages route 没有已核实的官方 WebSocket contract | `Unavailable` |
| DeepSeek | [Chat Completions](https://api-docs.deepseek.com/api/create-chat-completion) 的 streaming 为 data-only SSE | 当前 Chat route 不支持已核实的 WebSocket mode | `Unavailable` |
| Kimi | [Chat API](https://platform.kimi.ai/docs/api/chat) 的 streaming 为 SSE | Kimi OAuth 不改变该 wire contract | `Unavailable` |
| Ollama | [Streaming responses](https://docs.ollama.com/api/streaming) 使用 NDJSON | 当前 native/compatible route 不是 WebSocket | `Unavailable` |
| Hugging Face Router | [Chat completion streaming](https://huggingface.co/docs/inference-providers/en/tasks/chat-completion) 使用 SSE | Router Chat route 没有已核实的 WebSocket mode | `Unavailable` |
| Z.AI | [Streaming](https://docs.z.ai/guides/capabilities/streaming) 使用 SSE | 当前 GLM Chat route 没有已核实的 WebSocket mode | `Unavailable` |
| MiMo | [Responses API](https://mimo.mi.com/docs/en-US/api/chat/responses) 使用 SSE，且当前文档不支持 `previous_response_id` | 不能借用 OpenAI Responses WebSocket/session 假设 | `Unavailable` |
| Generic OpenAI-compatible | 没有统一上游 authority | HTTP path/JSON 兼容不证明 handshake、event lifecycle、sticky state 或 prewarm 兼容 | `Unavailable`，fail closed |

代码中的 `WebSocketApiProfile` 表达“Zeta 允许哪一种 exact wire codec”，不是“供应商公司是否在任意
产品里用过 WebSocket”。启用真实调用还需要 service target、model、credential scope、codec 与
session lifecycle 同时匹配；任一项未知就继续使用已验证的 HTTP route。

### 5.2 输入-token 计量矩阵

这里的“支持”只表示调用前 budget measurement，不是调用完成后的 response usage。所有 remote
binding 都由 `ProviderDefinition.input_token_count` 明确声明 profile、target 和 model policy；未声明
时 fail closed，不能因为 Chat Completions 外形兼容就推断存在 count endpoint。

| Provider | 官方计量面 | 当前 Zeta 状态 | 边界 |
| --- | --- | --- | --- |
| OpenAI | [`POST /responses/input_tokens`](https://developers.openai.com/api/reference/resources/responses/subresources/input_tokens) | ✅ exact remote | 与 Responses request 同 codec；直接读取 `input_tokens` |
| Anthropic | [`POST /v1/messages/count_tokens`](https://docs.anthropic.com/en/api/messages-count-tokens) | 部分具备：estimated remote | provider preflight 后按 1%/至少 32 tokens 保守记账 |
| Google | [`models.countTokens`](https://ai.google.dev/api/tokens) | 部分具备：estimated remote | native `generateContentRequest`；当前 invocation 是 OpenAI-compatible，且仅声明 model 可用 |
| Kimi | [`estimate-token-count`](https://platform.kimi.ai/docs/api/estimate) | 部分具备：estimated remote | 文档 schema 只有 model/messages；带 tools/reasoning 的当前请求退回 unavailable |
| Z.AI | [`POST /tokenizer`](https://docs.z.ai/api-reference/tools/tokenizer) | 部分具备：estimated remote | 使用 `usage.total_tokens`，支持 tools；带 Tool Call/Result 历史暂退 unavailable |
| xAI | [`tokenize-text`](https://docs.x.ai/developers/rest-api-reference/inference/other) | ❌ full-request preflight unavailable | 只 tokenize 裸文本；[billing FAQ](https://docs.x.ai/developers/faq/billing) 说明 inference 还会加入预定义 tokens |
| Qwen | [text generation](https://help.aliyun.com/en/model-studio/text-generation) | ❌ preflight unavailable | chat template 会增加控制 token，不能按裸文本计数 |
| DeepSeek | [token usage / offline tokenizer](https://api-docs.deepseek.com/quick_start/token_usage) | 部分具备：local/estimated | 已接入完整 `ModelRef` binding、请求级模板渲染与 tokenizer runtime；当前仍需宿主提供固定资产清单 |
| Ollama | [API usage fields](https://github.com/ollama/ollama/blob/main/docs/api.md) | ❌ preflight unavailable | `prompt_eval_count` 是调用完成后的 usage |
| Hugging Face Router | [per-model tokenizer API](https://huggingface.co/docs/tokenizers/main/api/tokenizer) | 部分具备：local/estimated | 公共 `owner/repo` 首次使用时解析 immutable commit，按需下载并校验 `tokenizer.json`、`tokenizer_config.json` 与 standalone template；重启复用磁盘缓存 |
| MiniMax | [Chat API](https://platform.minimaxi.com/docs/api-reference/text-post) | ❌ verified preflight unavailable | 当前官方文档只确认 response usage |
| MiMo | [Responses API](https://mimo.mi.com/docs/en-US/api/chat/responses) | ❌ verified preflight unavailable | 当前官方文档只确认 response `usage.input_tokens` |
| Generic OpenAI-compatible | 无统一标准 | ❌ unavailable | 必须由具体 provider definition 显式增加 count profile |

### 5.3 调用完成后的 token 使用量

调用完成后的统计统一进入 `ModelUsage`，但字段必须先按模型商官方定义换算，不能直接照搬同名
JSON 字段：

| 统一字段 | 含义 | 当前来源 |
| --- | --- | --- |
| `input_tokens` | 总输入，包含未缓存输入、缓存读取和缓存写入 | OpenAI 直接读取总输入；Anthropic 将三部分相加；DeepSeek 读取 `prompt_tokens` |
| `cached_input_tokens` | 从缓存读取的输入 token | OpenAI `cached_tokens`；Anthropic `cache_read_input_tokens`；DeepSeek `prompt_cache_hit_tokens` |
| `cache_write_input_tokens` | 写入缓存的输入 token | OpenAI `cache_write_tokens`；Anthropic `cache_creation_input_tokens`；DeepSeek 未报告时保持未知 |
| `output_tokens` | 总输出 | 各模型商的总输出字段 |
| `reasoning_tokens` | 总输出中的推理/思考明细 | 只在响应明确提供明细时记录 |

缓存占比定义为 `cached_input_tokens / input_tokens`，表示 token 维度的缓存读取占比，不表示“多少次
请求命中了缓存”。只有分子和分母都完整、且总输入大于 0 时才能给出精确百分比；否则显示未知。
Thread 和 Turn 聚合保留每项的 `complete`，某次调用缺少字段时只展示已报告下界，不补 0。

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

### 5.4 OpenAI 服务接口面不是 OpenAI-compatible 配置档案

OpenAI Platform API 与 ChatGPT 订阅服务都可能使用 `responses` 这样的相对 path，但它们的 base URL、credential、entitlement 和可用 operation 不能由 path 推断。详见 [`zeta-api.md`](zeta-api.md#45-openai-platform-与-chatgpt-订阅服务端点清单)。

因此 direct-provider runtime binding 包含如下事实：

```rust
pub struct OpenAiExecutionBinding {
    endpoint: zeta_api::ApiEndpoint,
    target: ResolvedApiTarget,
    credential_scope: CredentialScope,
}
```

ChatGPT 订阅不是 `OpenAiCompatibleAdapter` 的用户自定义 base URL 选项。Platform API key、custom-compatible credential 与 ChatGPT 订阅运行时彼此不能复用或降级转换。

ChatGPT 订阅由 [`zeta-chatgpt`](chatgpt-subscription.md) 构造固定 `ResolvedApiTarget + OAuth headers` binding。`zeta-model-provider` 使用同一个 OpenAI Responses adapter 执行单次模型 operation，Zeta Core 继续执行 Agent loop。

## 6. 供应商凭据边界

Provider 的共同点只到“调用前需要可用身份”为止。供应商提供并允许稳定的用户订阅 OAuth 时，账户生命周期必须通过 `zeta-login` 暴露；没有该能力但提供开发者 API 时，API key、AWS credential chain、Google ADC、Microsoft identity 和签名请求仍由本 crate 的 direct-provider runtime materialize。API key 不是 OAuth 登录方法或失败降级。交互式登录控制面不属于本 crate，但 provider-specific credential owner 可以向本 crate 提供已经刷新并绑定的 request target；Kimi Code 就使用这种窄边界。

ChatGPT 订阅使用本地 Agent loop：

```text
zeta-app-server → zeta-login → zeta-chatgpt → OpenAI device OAuth / SecretStore
                                      │
                                      └─ fresh ResolvedApiTarget
                                               ↓
Zeta TurnExecutor → zeta-model-provider → OpenAI Responses codec → ChatGPT subscription service
```

`runtime = chatgpt_subscription` row 的 provider 仍是 `openai`。运行时只选择 provider-specific authenticated target，不改变 Core backend，也不接受 arbitrary ChatGPT base URL。

Kimi Code 订阅使用本地 Agent loop：

```text
zeta-app-server → zeta-login → zeta-kimi → Kimi device OAuth / SecretStore
                                  │
                                  └─ fresh ResolvedApiTarget
                                           ↓
zeta-model-provider → KimiAdapter → zeta-api OpenAI Chat Completions → Kimi Coding API
```

目录中的 `kimi/kimi-k2.7-code` 是 `access = subscription, runtime = kimi_code`，请求时映射为 Kimi Coding API model `kimi-for-coding`。现有 `kimi/kimi-k2.6` 保持 `access = api_key, runtime = provider_api`，因此 Kimi Platform API key 与 Kimi subscription OAuth 没有 fallback、token 转换或 endpoint 混用。

401 recovery 也按身份所有者处理：direct-provider credential 可由其 provider runtime 做一次受限 refresh/rebuild；Kimi 与 ChatGPT token 分别由 `zeta-kimi`、`zeta-chatgpt` 在调用前按 expiry margin 刷新。`zeta-client` 不读取 secrets，也不自行刷新或重试认证。

## 7. 标头和目标

| 内容 | Owner |
| --- | --- |
| Platform/default provider base URL | `model-provider-config` |
| 用户 base URL override | 仅 Platform/custom-compatible provider；由 `model-provider-config` 声明、runtime 解析 |
| ChatGPT 订阅服务目标 | `zeta-chatgpt` 固定 target；不接受 Zeta generic user override |
| Kimi Code 订阅服务目标 | `zeta-kimi` 固定为 `https://api.kimi.com/coding/v1`；不接受 generic base URL override |
| resolved absolute target | `model-provider` |
| API key/cloud identity/tenant header | `model-provider` + direct-provider credential layer |
| relative path、method、content type | `zeta-api::endpoint` |
| API version、协议 beta header | `zeta-api::endpoint/requests` |
| trace propagation、user agent | `zeta-http-client` |
| operation attempt header | `zeta-client` |

Runtime 合并 headers 时必须使用 typed origin 和冲突规则。认证 header 不得被协议层覆盖，协议
必需 header 不得被用户任意删除；所有 secret header 的 `Debug` 必须脱敏。

## 8. 重试分工

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
- ChatGPT 订阅凭据刷新由 `zeta-chatgpt` 处理，不能套用 inference HTTP retry；
- 模型替换由 `zeta-models-manager` 在运行创建前依据 Agent、Session 或工作流策略完成，不是 client retry；
- runtime 只选择 typed policy，不自己 sleep 或写 attempt loop。

## 9. 流式处理分工

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

`ProviderDefinition.output_transport` 是该路径的 capability authority。OpenAI Responses、通用
OpenAI-compatible Chat、Google 与 Anthropic 声明 `nativeStreaming`；其他内置 adapter 当前声明
`unary` 并使用 final-response bridge。App Server 将 exact value 投影进 model catalog，Desktop 不按
provider 名称或协议 family 猜测。

`output_transport` 只区分当前 HTTP invocation 是否提供原生增量输出，不代表 WebSocket。
`websocket_api_profile` 是另一条 fail-closed capability：当前只有 OpenAI definition 声明
`OpenAiResponses`，且它只是后续 runtime binding 的必要条件，不表示 codec/session 已完成，也不能
替 ChatGPT subscription target 或自定义 compatible endpoint 作保证。

DeepSeek `: keep-alive` 的 frame 边界由 client 识别，作为无 payload 的 SSE comment 交给协议层；
协议层确认它不形成 model output。Anthropic `event: ping` 同样由协议层过滤。Runtime 不解析
`data:` 行或 provider event JSON。

## 10. 目录来源

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

ChatGPT 订阅的账户 metadata 只由 `zeta-chatgpt` 从已验证登录 token 投影；它不使 models manager 直读或猜测远端动态 catalog。

## 11. 依赖方向

箭头表示“依赖”：

```text
zeta-model-provider
  ├──▶ zeta-model-provider-config
  ├──▶ zeta-api ───▶ zeta-client
  ├──▶ zeta-client
  ├──▶ zeta-http-client
  ├──▷ zeta-websocket-client        [ModelClientSession 接入后]
  ├──▶ zeta-secrets
  └──▶ zeta-protocol

zeta-chatgpt
  ├──▶ zeta-login
  ├──▶ zeta-client
  └──▶ zeta-secrets

App Server composition
  ├──▶ zeta-model-provider
  ├──▶ zeta-login
  └──▶ zeta-chatgpt
```

更准确地说：

- `zeta-client` 不依赖 Provider、API codec、Core 或 config；
- `zeta-http-client` 不依赖 Provider、API codec、Core、config 或 secret store；
- `zeta-websocket-client` 只依赖共享 outbound network policy，不依赖 Provider、API codec、Core 或 secret store；
- `zeta-api` 可依赖 `zeta-client` 的 operation/SSE value；
- `zeta-model-provider` 依赖 config、API、operation client、HTTP client、secrets 和 protocol；
  它不定义或消费完整 Agent-loop backend；
- config 不反向依赖 runtime；
- API/client 都不反向依赖 model-provider。
- `zeta-chatgpt` 实现 interactive login、refresh 与 authenticated target；
- Zeta App Server 只组合和映射 redacted control-plane DTO；
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

## 13. 公共接口

Public API 只暴露：

- `ModelProvider`；
- `ModelProviderRuntime` 或更窄的 runtime factory；
- immutable `ModelInvoker`；
- `ModelRuntimeRequest`；
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
8. 建立 `zeta-login` 与 `zeta-chatgpt`，以 native ChatGPT OAuth 和本地 `TurnExecutor` 接入 subscription vertical slice；
9. 实现 models manager 的 catalog source；
10. 删除旧 Provider 级双重 dispatch。
11. 已建立独立 `zeta-websocket-client` 和显式 provider WebSocket profile；下一步在 `zeta-api`
    增加 Responses WebSocket codec，再由 model-provider 的 `ModelClientSession` 管理 connection、turn
    state、prewarm、`previous_response_id` 和 HTTP fallback。

迁移期间不创建空模块，也不同时保留两套长期 public facade。

## 15. 固定决策

1. Provider registry 只存在于 `zeta-model-provider`。
2. Runtime 选择 API endpoint，但不实现 wire codec。
3. Runtime 选择 retry policy，但 retry attempt loop 属于 `zeta-client`。
4. Direct-provider runtime 解析自己的 credential；secret 不进入 config、普通 inference DTO 或
   telemetry。ChatGPT 订阅凭据始终留在 `zeta-chatgpt` 的 SecretStore envelope。
5. Runtime 不解析 SSE/NDJSON framing。
6. 动态 catalog cache 属于 models manager。
7. `zeta-api` 和 `zeta-client` 不反向依赖 runtime。
8. direct-provider credential lifecycle 属于本 crate；`zeta-secrets` 只持久化 opaque secret。
9. interactive login lifecycle 属于 `zeta-login`；ChatGPT subscription OAuth wire 与 token lifecycle 属于 `zeta-chatgpt`。
10. 用户订阅 OAuth 统一进入 `zeta-login` 控制面，但 OAuth wire、token 和持久化仍由精确 provider adapter 拥有；无受支持 OAuth 的供应商才走 API key 等 direct-provider credential，二者不互相 fallback。
11. `model-provider` 只消费 ChatGPT authenticated target；产品始终使用 Zeta Core `TurnExecutor`。
12. WebSocket transport 不拥有 Agent loop；连接/session/turn routing 属于 model client，是否允许
    WebSocket 由 exact profile 与 runtime target capability 共同决定。
