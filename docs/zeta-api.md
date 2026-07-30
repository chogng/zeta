# 模型 API 协议

> - 物理位置：`zeta-rs/zeta-api/`
> - Rust crate：`zeta_api`
> - 层次：模型 API 协议层
> - 当前状态：同步 unary endpoint/profile dispatch、OpenAI Responses 与 Anthropic Messages SSE decoder 已迁移；live streaming invocation 尚未实现
> - Crate codec 与 decoder 实现：[`zeta-rs/zeta-api/README.md`](../zeta-rs/zeta-api/README.md)
> - Canonical contract：[`protocol.md`](protocol.md#6-provider-independent-model-contract)
> - Provider runtime：[`model-provider.md`](model-provider.md)
> - Operation client：[`zeta-client.md`](zeta-client.md)
> - 底层网络：[`zeta-http-client` README](../zeta-rs/http-client/README.md)
> - Direct-provider credential：[`model-provider.md`](model-provider.md#6-provider-credential-与-subscription-backend)
> - Secret persistence：[`secrets.md`](secrets.md)
> - Model catalog control plane：[`models-manager.md`](models-manager.md)
> - Subscription runtime adapter：[`codex-app-server.md`](codex-app-server.md)

> Provider 官方资料核对日期：2026-07-26。请求字段、事件类型、缓存语义和错误结构会持续变化；
> 实现必须以官方文档和脱敏 contract fixture 为准，不能仅凭 OpenAI-compatible 标签推断。

## 快速理解

`zeta-api` 是纯模型 API 协议层。它接收 provider-independent canonical model value，负责：

- 定义 API relative endpoint、HTTP method 和协议 headers；
- 将 canonical request 编码为具体 API request body；
- 将 unary response/error 解码为 canonical response/error；
- 将已经完成 framing 的 SSE event 解码为 canonical stream event；
- 解释 Provider 级 terminal、heartbeat、usage、tool、reasoning 和 prompt-cache 字段；
- 提供 catalog endpoint 的 request/response codec。

它不再拥有 Provider registry，也不再实现 transport、retry、SSE framing 或 telemetry。

目标横向结构是：

```text
endpoint/    requests/    sse/
```

三者同级：

- `endpoint/` 描述 method、relative path、required headers 和所绑定的 codec；
- `requests/` 描述 request/unary response/error JSON；
- `sse/` 描述具体 API 的 SSE event schema、lifecycle 和 canonical assembly。

通用 HTTP/WebSocket backend、proxy/TLS/pool 与 transport diagnostics 属于 `zeta-http-client`；
operation retry、SSE/NDJSON framing 和 operation telemetry 属于 `zeta-client`。

| 需要处理的内容 | 本层是否负责 | 交给谁 |
| --- | --- | --- |
| 把统一模型请求编码成供应商 JSON | ✅ | 本层 |
| 解释供应商响应、错误和流式事件 | ✅ | 本层 |
| 选择供应商、模型和凭据 | ❌ | 模型调用系统 |
| 判断是否安全重试并执行等待 | ❌ | 模型调用操作层 |
| 建立 HTTP 连接、代理和 TLS | ❌ | 网络层 |
| 推进 Agent Turn 和工具循环 | ❌ | 会话与执行系统 |

## 2. 四层关系

```text
zeta-model-provider-config
  声明 Provider、默认 base URL、允许的 API profile
                │
                ▼
zeta-model-provider
  解析 credential/target，选择 API endpoint 和 retry policy
                │
                ▼
zeta-api
  endpoint + requests + SSE protocol codec
                │
                ▼
zeta-client
  operation retry + framing + telemetry
                │
                ▼
zeta-http-client
  HTTP/WebSocket execution + network policy
```

依赖与控制流不是同一个方向。Rust 依赖建议为：

```text
zeta-http-client
       ▲
       │
zeta-client      zeta-protocol
      ▲              ▲
      └──── zeta-api ─┘
               ▲
               │
      zeta-model-provider
```

规则：

- `zeta-client` 不依赖 `zeta-api`；
- `zeta-http-client` 不依赖 `zeta-client` 或 `zeta-api`；
- `zeta-api` 可以使用 operation client 的 request/response/SSE value；
- `zeta-api` 不依赖 model-provider/config/models-manager/Core；
- model-provider 选择 `zeta-api` endpoint 并注入 resolved runtime values；
- Provider registry 只存在于 model-provider。

## 3. 当前实现审计

当前 crate 已实现：

- provider-independent `ApiEndpoint` 和 `ApiProtocol`；
- `endpoint/`、`requests/`、`sse/` 三个顶级 codec 模块；
- OpenAI Responses unary codec；
- Anthropic Messages unary codec；
- OpenAI-compatible Chat Completions unary codec；
- OpenAI Responses 已 framed SSE event → canonical text/reasoning delta decoder；
- Anthropic Messages 已 framed SSE lifecycle decoder（text/thinking delta、`ping`、content block close、terminal validation）；
- 基础 text/tool/reasoning/usage/stop reason 映射；
- API endpoint fixtures；
- 对当前 `zeta-client::HttpClient` unary byte port 的临时依赖；raw port 后续迁入
  `zeta-http-client`。

需要修正：

| 当前设计 | 目标 |
| --- | --- |
| legacy provider-named codec source files | 已删除；三份 unary codec 已物理移动到 `requests/` |
| `JsonHttpTransport` / `UreqJsonHttpTransport` | 已替换为 `ClientRequest`/`ClientResponse` 与 `HttpClient` |
| `ResolvedApiTarget` 同时承担 runtime 和协议职责 | 仍需将其演进为 typed client request；API 只追加协议 path/header |
| transport 直接返回 `serde_json::Value` | client 已返回 status/headers/body bytes，API 负责 JSON |
| provider facade 与 wire codec 两套目录 | Provider facade 已只留在 model-provider；API dispatch 只按 endpoint/profile |

当前代码仍是同步、非流式 unary。目标文档不能被理解为 streaming 已经实现。

## 4. `endpoint / requests / sse`

### 4.1 `endpoint/`

Endpoint 是一个具体在线 operation contract，例如：

- OpenAI `POST /v1/responses`；
- OpenAI Chat `POST /v1/chat/completions`；
- Anthropic `POST /v1/messages`；
- Gemini `POST ...:generateContent`；
- Gemini `POST ...:streamGenerateContent?alt=sse`；
- Ollama `POST /api/chat`；
- Anthropic `GET /v1/models`；
- Ollama `GET /api/tags`。

Endpoint 拥有：

```text
HTTP method
relative path/path builder
query schema
protocol-required headers
request media type
expected response media type
request codec identity
unary response codec identity
stream codec identity
operation retry evidence
```

Endpoint 不拥有：

- 默认 base URL；
- credential headers；
- proxy、redirect 或 connection pool；
- retry attempt loop；
- JSON DTO implementation；
- SSE byte framing。

Endpoint path 必须相对。它不能替换 resolved target 的 scheme/host，也不能通过字符串修剪猜测另一
API 的地址。

### 4.2 `requests/`

`requests/` 负责 JSON/body 层：

- canonical `ModelRequest` → wire request；
- typed invocation option → wire field/header intent；
- unary body → canonical `ModelResponse`；
- HTTP error body → typed API error evidence；
- usage、tool、content、reasoning 和 stop reason；
- prompt/context cache 参数和 usage；
- catalog request/response/pagination DTO。

每个 request codec 必须对 canonical intent 做三选一：

1. 准确编码；
2. 以文档化等价语义编码；
3. 返回 typed `UnsupportedFeature` / `InvalidRequest`。

不能静默丢弃 tool choice、reasoning、image、strict schema、parallel tool call 或 cache intent。

### 4.3 `sse/`

`sse/` 不处理 TCP chunk、换行或 `data:` 拼接。它消费 `zeta-client` 已完成 framing 的
`SseFrame`：

```rust
/// Decodes already-framed SSE values for one concrete API profile.
///
/// Implementations validate provider event lifecycles and emit only canonical model events.
pub trait SseDecoder {
    type State;

    fn decode(
        &self,
        state: &mut Self::State,
        frame: &zeta_client::SseFrame,
    ) -> Result<Vec<ModelStreamEvent>, ApiError>;
}
```

名称仅表达目标语义。具体 API 可以使用 enum/associated type，避免不必要的 dynamic dispatch。

`sse/` 拥有：

- event name 和 `data` JSON schema；
- Provider event lifecycle；
- `ping`/comment/terminal 的协议解释；
- text/reasoning/tool arguments/usage delta；
- 未知 optional event 的 forward compatibility；
- 跨 event canonical assembler；
- EOF 时 terminal validation；
- 最终 response 与 unary response 的语义一致性。

`sse/` 不拥有：

- HTTP connection；
- retry/backoff；
- byte buffer；
- CRLF/LF 和 multiline `data` framing；
- idle timer；
- transport metrics backend。

### 4.4 非 SSE 流

Ollama native API 使用 NDJSON，不能塞进名为 `sse/` 的模块。主架构仍以用户确定的三条同级主轴
为中心，但遇到已验证的非 SSE 协议时增加同级模块：

```text
endpoint/    requests/    sse/    websocket/    ndjson/
```

`ndjson/` 只解释由 `zeta-client::NdjsonRecord` 完成 framing 的 API object。未来若接入 WebSocket
或 gRPC，也按真实协议增加同级 codec，不能伪装成 SSE。

### 4.5 OpenAI Platform 与 ChatGPT/Codex 端点清单

此前把候选名仅与公开 OpenAI Platform `/v1` OpenAPI 比对，遗漏了 ChatGPT/Codex service
surface。这是错误的。`../codex/codex-rs/codex-api/src/endpoint/mod.rs` 的 endpoint 集合明确包含
`compact`、`images`、`memories`、`models`、`realtime_call`、`realtime_websocket`、`responses`、
`responses_websocket` 与 `search`。

它们不能视为同一条 API：公开 Platform API 通常以 `https://api.openai.com/v1` 为 base，而 Codex
service 使用 ChatGPT 登录态和独立的 Codex base。Zeta 不把后者变成一个 `zeta-api` API surface：
`zeta-api` 的 OpenAI endpoint binding 只描述公开 Platform API，Codex subscription 交给其独立
runtime adapter。

```rust
/// Selects a public OpenAI API surface with its own base URL,
/// credential kind, headers, and supported endpoint set.
pub enum OpenAiServiceSurface {
    PlatformApi,
}
```

`zeta-model-provider` 解析 `OpenAiServiceSurface` 和 direct-provider credential；Codex subscription
则由 [`zeta-codex-app-server`](codex-app-server.md) 持有其独立 target/auth/runtime。`zeta-api` 仅根据
已选 direct API surface 编解码对应 contract。绝不能用 Platform API key 访问 Codex profile，也不能让
任意 custom OpenAI-compatible URL 冒充 ChatGPT/Codex service。Codex 官方的
[Memories 文档](https://developers.openai.com/codex/memories)也确认：本地 Codex memory store 是
独立能力；远端 summarization operation 只是其生成路径的一部分，不是 memory 的权威存储。

ChatGPT subscription 登录与其 backend contract 不进入本 crate：`zeta-codex-app-server` 通过
上游 Codex App Server 执行。特别禁止复制 `codex-api/src/endpoint/session.rs`，也禁止让
`zeta-api` 接收 Codex token、ChatGPT account routing header 或私有 backend URL。

上游模块是能力清单，不是 Zeta 要直接复刻的 HTTP endpoint 清单：

| 上游 capability | Zeta 长期接入点 |
| --- | --- |
| ChatGPT login、token refresh、account/rate-limit、Codex thread/turn | `zeta-codex-app-server`，通过上游 JSON-RPC |
| Codex responses/stream、approvals、compact | `SubscriptionModelBackend` 的 semantic mapping；不引入 ChatGPT backend URL 或 header |
| memories、search、images、realtime | 只有 Zeta 的产品需求和上游公开 App Server contract 同时满足时，才在 Codex adapter 增加单独 vertical slice |
| OpenAI Platform Responses/Chat/Images/Realtime | 继续按公开 Platform API 在 `zeta-api` 编解码 |

上游 `responses_websocket`、`realtime_websocket` 的存在不等于 Zeta 应直连其 socket。对 subscription
路径，连接与 protocol 由 upstream Codex 承担；对公开 Platform API，才按真实协议在
`zeta-api::websocket`、`zeta-client` 和 `zeta-http-client` 分层实现。

## 5. 供应商运行时联动

`zeta-api` 不知道当前 Provider registry，但可以提供包含兼容差异的 endpoint profile：

```text
model-provider::providers::deepseek
  → 选择 OpenAI Chat endpoint
  → 选择 DeepSeek-compatible request/SSE/error profile

model-provider::providers::xai
  → 显式选择 Responses 或 Chat endpoint

model-provider::providers::google
  → 显式选择 Interactions、GenerateContent 或 compatible Chat
```

Provider-specific wire quirk 的实现仍属于 API 协议层，例如：

- DeepSeek prompt cache hit/miss usage；
- DeepSeek SSE comment heartbeat；
- Anthropic `ping` 和 content block lifecycle；
- Z.AI HTTP/business error 双层状态；
- MiniMax `base_resp`；
- Ollama NDJSON `done`/`error`。

但选择哪个 quirk/profile 的责任属于 model-provider。`zeta-api` 不通过 Provider ID、URL 或 model
name 自行选择。

可以用 typed profile：

```rust
pub enum OpenAiChatProfile {
    Baseline,
    DeepSeek,
    Xai,
    QwenCompatible,
    Zai,
    MiniMax,
}
```

这个 enum 表达 wire 差异，不是 Provider registry。没有实质差异的 profile 不需要提前创建。

## 6. 与 `zeta-client` 的边界

### 6.1 请求

```text
model-provider
  ResolvedTarget + runtime headers
        │
zeta-api::endpoint
  relative path + protocol headers
        │
zeta-api::requests
  body bytes
        │
zeta-client::ClientOperation
        │
zeta_http_client::HttpRequest
```

Header ownership：

| Header | Owner |
| --- | --- |
| Authorization、API key、tenant/deployment | model-provider/credential runtime |
| Content-Type、Accept | zeta-api endpoint |
| Anthropic version/beta feature | zeta-api endpoint/typed request |
| traceparent、tracestate、user-agent | zeta-http-client |

Header merge 必须有冲突规则，禁止简单拼接后让后写值静默覆盖 secret 或协议 header。

### 6.2 Unary 响应

`zeta-client` 通过 `zeta-http-client` 返回：

```text
status + headers + bounded bytes + attempt/timing evidence
```

`zeta-api::requests` 决定：

- success JSON schema；
- error JSON schema；
- HTTP 200 body 是否仍表示业务错误；
- request ID/retry evidence 的安全提取；
- canonical response/error。

### 6.3 流式处理响应

```text
zeta-client
  bytes → SseFrame/NdjsonRecord
        │
zeta-api::sse/ndjson
  frame → canonical ModelStreamEvent
```

底层 transport idle timer 在任意合法 wire activity 时更新；operation client 维护 frame activity。
API decoder 维护 semantic progress，并过滤不应暴露为模型输出的 heartbeat。

### 6.4 重试

`zeta-client` 执行 retry，`zeta-api` 只提供协议事实：

- HTTP status；
- `Retry-After`；
- Provider error code/type；
- 是否出现 terminal/semantic event；
- operation 是否有文档化 idempotency evidence。

Model-provider 选择最终 typed policy。API 不 sleep、不创建 attempt loop，也不 fallback。

### 6.5 遥测

`zeta-http-client` 拥有 HTTP/WebSocket diagnostics 与 redaction；`zeta-client` 拥有
operation/attempt/stream telemetry。API 只提供低基数 protocol classification：

```text
api.profile
api.operation
api.result_class
api.stream.event_class
api.stream.heartbeat_class
```

API 不记录 raw request/response/SSE payload。Exact model ID、prompt、tool arguments 和 secret 不得
进入默认 telemetry。

## 7. Canonical 契约

Canonical values 由 `zeta-protocol` 拥有。`zeta-api` 只引用或显式 re-export 必要类型：

```text
ModelRequest
├── instructions
├── input: Message | ToolResult
├── tools / tool choice
├── reasoning intent
├── output limit
└── sampling intent

ModelResponse
├── output: Text | Refusal | Reasoning | ToolCall
├── usage
└── stop reason
```

以下内容不进入 protocol：

- Provider JSON DTO；
- HTTP status/header/URL；
- SSE event name、NDJSON object；
- Provider raw error；
- prompt-cache 原始字段；
- profile compatibility toggle。

只有两个以上独立组件需要相同语义时，才将新 value 提升到 protocol。

## 8. 流式处理、heartbeat 与完成语义

状态机：

```text
Created
  → Decoding
       ├─ Heartbeat*
       ├─ SemanticEvent*
       └─ Terminal
  → Completed
  → Failed
  → Cancelled
```

分工示例：

| Wire input | `zeta-client` | `zeta-api` | Canonical output |
| --- | --- | --- | --- |
| Anthropic `event: ping` | 生成 `SseEvent`、更新 frame activity | 识别为 heartbeat | 无 |
| DeepSeek `: keep-alive` | 生成 comment frame、更新 activity | profile 确认为 liveness | 无 |
| `data: [DONE]` | 生成 data event | profile 校验 terminal | `Completed` |
| OpenAI typed delta | 生成 data event | 解码/组装 | text/tool/usage delta |
| Ollama JSON line | 生成 NDJSON record | 解码 `done/error` | delta/completed/error |

EOF 不自动等于成功。需要文档化 terminal event/marker 的 API，在 terminal 之前 EOF 必须返回
truncated stream。

`last_wire_activity` 属于 client；`last_semantic_progress` 属于 API/runtime。不能因为模型长时间
reasoning 而把“无文本”误判为连接死亡。

## 9. Prompt/上下文缓存

Prompt cache 的 wire 语义属于 `requests/`：

- OpenAI prompt cache key/retention/breakpoint；
- Anthropic `cache_control`、`5m`/`1h` 和 creation/read usage；
- Gemini implicit cache 和 explicit cached-content reference；
- xAI Chat header 与 Responses body key 的差异；
- DeepSeek/Qwen/Z.AI 自动缓存 usage。

不能强行统一为一个 `cache: bool`。Canonical contract 可以表达最小 provider-independent intent，
精确配置使用 typed profile option。

Ollama `keep_alive` 是模型 residency request 字段，不是 prompt cache，也不是 stream heartbeat。

Models manager 只把 cache capability 当 metadata，不参与一次 invocation 的 cache 生命周期。

## 10. 目录协议格式

Catalog request/response codec 也按同一结构组织：

```text
endpoint/anthropic/models.rs
requests/catalog/anthropic.rs
```

API 层负责：

- GET path/query/header；
- 分页 token 保真；
- JSON DTO；
- response headers 中 ETag/Last-Modified/Cache-Control evidence；
- Provider-neutral observation page。

它不负责 refresh、TTL、SWR、singleflight、merge、filter 或 availability interpretation。调用链：

```text
models-manager::ModelCatalogSource
        ▲
model-provider runtime adapter
        │
zeta-api catalog codec
        │
zeta-client
```

Inference endpoint 与 catalog endpoint 可能不共享 base URL，model-provider 必须显式解析 target。

## 11. 错误

目标错误只表达协议事实：

```rust
pub enum ApiError {
    InvalidRequest(InvalidRequestError),
    UnsupportedFeature(UnsupportedFeatureError),
    Provider(ProviderError),
    InvalidResponse(InvalidResponseError),
    InvalidStream(InvalidStreamError),
    Client(zeta_client::ClientError),
}
```

所有权：

- `zeta-http-client`：DNS/TCP/TLS/proxy/HTTP/attempt deadline/cancellation；
- `zeta-client`：operation deadline/retry/framing；
- `zeta-api`：Provider error body/event、invalid JSON、invalid lifecycle；
- model-provider：Provider/model/target identity 和 runtime resolution；
- Agent runtime：retry/fallback/用户提示/Turn outcome；
- App Server：stable product error mapping。

Provider error message 默认不是稳定公共 API，不能未经清洗直接展示或记录。

## 12. 目标目录

`endpoint / requests / sse` 是同级主目录；当已验证的 API 使用另一种 wire protocol 时，才增加
`websocket/` 或 `ndjson/` 这样的同级 codec 目录：

```text
zeta-rs/zeta-api/
├── BUILD.bazel
├── Cargo.toml
├── README.md
├── src/
│   ├── lib.rs
│   ├── endpoint/
│   │   ├── mod.rs
│   │   ├── openai_platform/
│   │   │   ├── mod.rs
│   │   │   ├── responses.rs
│   │   │   ├── compact.rs
│   │   │   ├── responses_websocket.rs
│   │   │   ├── realtime_websocket.rs
│   │   │   ├── models.rs
│   │   │   ├── images.rs
│   │   │   ├── conversations.rs
│   │   │   ├── vector_store_search.rs
│   │   │   └── realtime/{mod,calls,session}.rs
│   │   ├── anthropic/{messages,models}.rs
│   │   ├── gemini/{interactions,generate_content,models}.rs
│   │   ├── ollama/{chat,tags}.rs
│   │   └── endpoint_tests.rs
│   │
│   ├── requests/
│   │   ├── mod.rs
│   │   ├── openai_platform/
│   │   │   ├── responses/
│   │   │   │   ├── mod.rs
│   │   │   │   ├── request.rs
│   │   │   │   ├── response.rs
│   │   │   │   ├── error.rs
│   │   │   │   ├── usage.rs
│   │   │   │   ├── cache.rs
│   │   │   │   ├── tools.rs
│   │   │   │   └── request_tests.rs
│   │   │   ├── compact.rs
│   │   │   ├── images/{mod,generation,edit,variation}.rs
│   │   │   └── realtime/calls.rs
│   │   ├── anthropic/messages/
│   │   ├── gemini/{interactions,generate_content}/
│   │   ├── ollama/chat/
│   │   └── catalog/
│   │       ├── anthropic.rs
│   │       ├── gemini.rs
│   │       ├── openai.rs
│   │       ├── deepseek.rs
│   │       └── ollama.rs
│   │
│   ├── sse/
│   │   ├── mod.rs
│   │   ├── assembler.rs
│   │   ├── openai_platform/{responses,chat}.rs
│   │   ├── anthropic/messages.rs
│   │   ├── gemini/{interactions,generate_content}.rs
│   │   └── sse_tests.rs
│   │
│   ├── websocket/
│   │   ├── mod.rs
│   │   ├── openai_platform/responses.rs
│   │   └── websocket_tests.rs
│   │
│   ├── ndjson/
│   │   ├── mod.rs
│   │   ├── ollama/chat.rs
│   │   └── ndjson_tests.rs
│   │
│   ├── options/
│   │   ├── mod.rs
│   │   ├── profile.rs
│   │   ├── cache.rs
│   │   └── options_tests.rs
│   └── error/
│       ├── mod.rs
│       ├── invalid_request.rs
│       ├── unsupported_feature.rs
│       ├── provider.rs
│       ├── invalid_response.rs
│       ├── invalid_stream.rs
│       └── error_tests.rs
└── tests/
    ├── endpoint_contracts.rs
    ├── final_response_parity.rs
    └── fixtures/
        ├── openai/
        ├── anthropic/
        ├── google/
        ├── deepseek/
        └── ollama/
```

`endpoint/`、`requests/`、`sse/` 按职责横向分离，但相同 API profile 的命名必须一致，测试通过
fixture 把三者重新绑定。目录不是创建空文件的要求；只有实现 vertical slice 时才新增模块。

任何 Rust module 接近 500 LoC 时按 request/response/error/usage/cache 拆分，超过约 800 LoC
不继续堆功能。新 test module 使用 sibling `*_tests.rs`。

## 13. 公共接口

只导出：

- endpoint/profile selector；
- typed invocation options；
- canonical encode/decode entry point；
- streaming decoder/handle 所需窄接口；
- catalog observation page；
- structured `ApiError`；
- 必要的 canonical value re-export。

不导出：

- Provider registry；
- concrete HTTP client；
- raw Provider DTO；
- 任意 JSON option；
- telemetry backend；
- credential/header store。

新增 public trait 必须说明实现者对 validation、terminal lifecycle、unknown event、secret redaction
和 final-response parity 的责任。

## 14. 供应商/配置档案验证矩阵

| Provider runtime | API profile | Streaming | 已确认的协议差异 |
| --- | --- | --- | --- |
| OpenAI | Responses | typed Responses SSE | terminal event、prompt cache usage |
| ChatGPT/Codex subscription | upstream Codex App Server semantic runtime | upstream event mapping | 不在 `zeta-api` 直连或编解码；由 `zeta-codex-app-server` adapter 验证 |
| Anthropic | Messages | named Messages SSE | `ping`、content-block lifecycle、cache creation/read |
| OpenAI-compatible | Chat Completions | data SSE + `[DONE]` baseline | 只保证最小 configured contract |
| Google | Interactions / GenerateContent / compatible Chat | typed SSE / GenerateContent SSE | profile 不得静默互换 |
| xAI | Responses / Chat | 对应 typed SSE | cache routing 在两个 profile 中不同 |
| Qwen | compatible Chat / DashScope native | 对应 stream contract | compatible 与 `X-DashScope-SSE` 不混用 |
| DeepSeek | Chat | Chat SSE + comment heartbeat | keep-alive、hit/miss usage、unary whitespace |
| Ollama | native Chat / compatible Chat | NDJSON / Chat SSE | `keep_alive` 是 residency |
| Hugging Face | routed Chat | Chat SSE baseline | router/downstream error 需独立验证 |
| Z.AI | Chat | compatible SSE | HTTP/business code、stream finish reason |
| MiniMax | Anthropic Messages / Chat | 两种 SSE | `base_resp`、profile 显式选择 |

`Unknown` 不等于“不支持”，也不等于“与 OpenAI 相同”。未取得官方文档或 fixture 的高级行为不向
上声明。

## 15. 测试

### 15.1 端点

- method、relative path、query；
- required protocol headers；
- header 冲突和 secret redaction；
- expected response media type；
- endpoint/profile 绑定；
- custom target 不能被 endpoint 替换 origin。

### 15.2 请求

- canonical request → exact wire fixture；
- unary fixture → canonical response；
- tool/reasoning/image/usage/stop reason；
- prompt cache 参数和 usage；
- unsupported intent 不静默丢弃；
- HTTP 200 business error；
- malformed/oversized/unknown response；
- catalog pagination token 保真。

### 15.3 SSE/NDJSON

- 从 `zeta-client::SseFrame` 开始测试，不在 API crate 重复 byte fragmentation 测试；
- text/reasoning/tool argument 顺序；
- Anthropic `ping`；
- DeepSeek comment heartbeat；
- `[DONE]` 和 typed terminal event；
- unknown optional event；
- stream error event；
- EOF before terminal；
- final assembled response 与 unary fixture 等价；
- Ollama `done/error` NDJSON object。

Byte fragmentation、CRLF/LF、multiline data 和 retry timing 的测试属于 `zeta-client`；transport
idle deadline、proxy/TLS、pool 和 HTTP diagnostics 的测试属于 `zeta-http-client`。

## 16. 迁移计划

### 阶段 1：建立共享客户端分层

- 在 `zeta-http-client` 建立 HTTP request/response/config port；
- 把 `HttpHeader`、raw request/response bytes 和 `UreqHttpClient` 迁入底层 crate；
- `zeta-client` 保留 operation retry 与 framing；
- 保住现有同步 unary 行为；
- `zeta-api` 开始依赖 operation client port。

### 阶段 2：拆端点与请求

- 已将 `Api` Provider enum 的分派移到 model-provider；
- 已建立 OpenAI Responses、Anthropic Messages、OpenAI Chat endpoint；
- 已把现有 JSON conversion 物理移入 `requests/`；
- 已删除 Provider 级空 facade。

### 阶段 3：SSE 纵向切片

- 在 client 实现 SSE framing；
- 已在 API 实现 OpenAI Responses `sse/` decoder；
- 已接 Anthropic Messages decoder 与 `ping`/content-block lifecycle；
- 由 model-provider 暴露 canonical stream。

### 阶段 4：兼容差异与 NDJSON

- DeepSeek comment heartbeat/usage；
- Ollama native NDJSON；
- Google native profiles；
- 逐家补 error/cache/stream fixtures。

### 阶段 4.5：ChatGPT/Codex 服务接口面

- 建立 [`zeta-codex-app-server`](codex-app-server.md)，由 upstream Codex managed login 提供
  ChatGPT/Codex subscription runtime；
- 将 Platform API key、Codex subscription runtime 和 custom-compatible target 设为互斥 binding；
- 仅在 upstream App Server 的公开 JSON-RPC 支持范围内接入 models、thread/turn 和 stream；
- 为 realtime call / WebSocket 三套 wire adapter 建立脱敏 contract fixture；
- 不复制 Codex `EndpointSession` 的 transport/retry/telemetry 所有权。

### 阶段 5：目录协议格式

- 实现 list endpoint/request/response codec；
- model-provider 实现 models manager source；
- manager 负责 refresh/cache/merge。

## 17. 固定决策

1. `zeta-api` 是协议层，不是 Provider registry。
2. `endpoint/`、`requests/`、`sse/` 是同级主目录；已验证的 WebSocket/NDJSON protocol 才新增同级 codec。
3. 非 SSE 协议使用真实模块，例如 `websocket/`、`ndjson/`。
4. Raw transport 与 network policy 属于 `zeta-http-client`；retry、SSE framing 和 operation
   telemetry 属于 `zeta-client`。
5. `zeta-api::sse` 只解释已经 framed 的 API event。
6. Provider/profile 选择属于 `zeta-model-provider`。
7. Config 只声明 profile，不持有 runtime API object。
8. Canonical values 属于 `zeta-protocol`。
9. Prompt cache wire mapping 属于 `requests/`，catalog cache 属于 models manager。
10. Inference retry safety 由 runtime policy 显式选择，client 执行。
11. Provider error 不以 raw JSON/String 穿透产品 API。
12. 当前没有 live stream HTTP execution 或 `ModelInvoker` stream API。`zeta-client` 的 SSE framer
    与 OpenAI Responses event decoder 都有单元测试，但这不等同于 provider streaming 已接通。
13. `OpenAiServiceSurface` 只用于公开 Platform API endpoint binding；ChatGPT/Codex subscription
    不共享 base URL、credential 或 custom endpoint override，而是独立 runtime。
14. ChatGPT/Codex subscription auth 与 backend protocol 属于 upstream Codex App Server；本 crate
    不定义其 OAuth wire codec、token/header value 或私有 backend target。

## 18. 官方资料索引

### OpenAI

- [Responses streaming](https://developers.openai.com/api/docs/guides/streaming-responses)
- [Prompt caching](https://developers.openai.com/api/docs/guides/prompt-caching)
- [Models](https://developers.openai.com/api/docs/models)
- [Codex Memories](https://developers.openai.com/codex/memories)
- [Codex authentication](https://learn.chatgpt.com/docs/auth)
- [Codex app-server](https://learn.chatgpt.com/docs/app-server)
- 本地 Codex source snapshot：`../codex/codex-rs/codex-api/src/endpoint/`（验证
  `memories/trace_summarize`、`alpha/search`、Responses/Realtime WebSocket 和 service target contract）

### Anthropic

- [Streaming Messages](https://platform.claude.com/docs/en/build-with-claude/streaming)
- [Prompt caching](https://platform.claude.com/docs/en/build-with-claude/prompt-caching)
- [API errors](https://platform.claude.com/docs/en/api/errors)
- [List models](https://platform.claude.com/docs/en/api/models/list)

### Google Gemini

- [Interactions and text generation](https://ai.google.dev/gemini-api/docs/text-generation)
- [`streamGenerateContent`](https://ai.google.dev/api/generate-content)
- [Models API](https://ai.google.dev/api/models)
- [Context caching](https://ai.google.dev/gemini-api/docs/caching/)

### xAI

- [Streaming](https://docs.x.ai/developers/model-capabilities/text/streaming)
- [Prompt caching](https://docs.x.ai/developers/advanced-api-usage/prompt-caching)
- [Model APIs](https://docs.x.ai/developers/rest-api-reference/inference/models)

### Qwen / Alibaba Cloud 模型 Studio

- [Streaming output](https://help.aliyun.com/en/model-studio/stream)
- [Context cache](https://help.aliyun.com/en/model-studio/context-cache)
- [Models and regional endpoints](https://help.aliyun.com/en/model-studio/models)

### DeepSeek

- [Chat Completions](https://api-docs.deepseek.com/api/create-chat-completion)
- [Rate limit and request keep-alive](https://api-docs.deepseek.com/quick_start/rate_limit)
- [List models](https://api-docs.deepseek.com/api/list-models)
- [Context caching](https://api-docs.deepseek.com/news/news0802/)

### Ollama

- [Native chat](https://docs.ollama.com/api/chat)
- [NDJSON streaming](https://docs.ollama.com/api/streaming)
- [Streaming errors](https://docs.ollama.com/api/errors)
- [List installed models](https://docs.ollama.com/api/tags)

### 其他

- [Hugging Face Chat Completion](https://huggingface.co/docs/inference-providers/tasks/chat-completion)
- [Z.AI Chat Completion](https://docs.z.ai/api-reference/llm/chat-completion)
- [Z.AI context caching](https://docs.z.ai/guides/capabilities/cache)
- [MiniMax OpenAI-compatible API](https://platform.minimax.io/docs/api-reference/text-openai-api)
- [MiniMax Anthropic-compatible API](https://platform.minimax.io/docs/api-reference/text-anthropic-api)

Kimi 与 MiMo 保持 configured compatible endpoint，直到取得完整官方 streaming/cache/catalog
reference 或经授权的脱敏 contract fixture。
