# `zeta-client` 架构与演进方案

> - 物理位置：`zeta-rs/zeta-client/`
> - Rust crate：`zeta_client`
> - 层次：Zeta API operation client、retry 与 stream framing
> - 当前状态：typed unary request/response、安全 retry loop、增量 SSE framer 和 telemetry wrapper
>   已实现；底层 HTTP port 与 production backend 已在 `zeta-http-client`
> - 底层网络：[`zeta-http-client` README](../zeta-rs/http-client/README.md)
> - API 协议层：[`zeta-api.md`](zeta-api.md)
> - Provider runtime：[`model-provider.md`](model-provider.md)

> 标准依据：
> [WHATWG Server-sent events](https://html.spec.whatwg.org/dev/server-sent-events.html)、
> [RFC 9110 HTTP Semantics](https://www.rfc-editor.org/rfc/rfc9110.html)、
> [W3C Trace Context](https://www.w3.org/TR/trace-context/) 和
> [OpenTelemetry HTTP semantic conventions](https://opentelemetry.io/docs/specs/semconv/http/)。

## 1. 结论

`zeta-client` 是 Provider-neutral 的 API operation client。它把调用方已经构造完成的 request
通过共享 `zeta-http-client` 执行，并统一处理：

- operation retry safety、classifier、attempt loop、backoff、jitter 和 `Retry-After`；
- 包含全部 attempt/backoff 的 overall operation deadline；
- SSE 与 NDJSON framing；
- wire/frame activity 与 idle policy 的组合；
- framed stream backpressure；
- operation、attempt 和 stream telemetry。

它不创建 HTTP backend，也不拥有 proxy、TLS、证书、redirect、连接池或 transport logging。
这些能力统一属于 [`zeta-http-client`](../zeta-rs/http-client/README.md)，本文件不重复其规则。

它也不知道 OpenAI、ChatGPT、Codex、Anthropic 或任何 `ModelRequest`。Provider JSON、event 和
error 语义属于 `zeta-api`。

```text
model-provider
  ├── resolve target/auth/retry safety
  ├──▶ zeta-api encode
  └──▶ zeta-client operation
          ├── retry/framing/operation telemetry
          └──▶ zeta-http-client
                  └── proxy/TLS/redirect/timeout/pool/backend

zeta-client 负责“怎样执行并组织一个 Zeta API operation”
zeta-api    负责“这个 API 的 bytes/event 表示什么”
```

当前实现仍直接导出 `HttpHeader`、`ClientRequest`/`ClientResponse`、`HttpClient` 和
`UreqHttpClient`。这是新底层 crate 落地前的迁移状态，不是长期 ownership；现有
`RetryPolicy`、SSE framer 和 operation telemetry 是保留在 `zeta-client` 的能力。

## 2. 拥有与不拥有

### 2.1 拥有

- operation request context；
- typed replay safety 和 retry policy；
- retry classifier、attempt state machine、backoff、jitter 与 `Retry-After`；
- operation overall deadline 和 attempt budget 分配；
- WHATWG SSE framing；
- NDJSON record framing；
- UTF-8、CRLF/LF、chunk boundary 和 frame/record hard limit；
- byte/frame activity timestamp；
- framed stream bounded channel 和 backpressure；
- attempt、retry、framing 与 stream operation telemetry；
- transport facts 到 operation error 的安全映射。

### 2.2 不拥有

- HTTP backend、DNS/TCP、proxy、TLS、证书、redirect 或连接池；
- transport attempt timeout、raw response body limit 或 HTTP redaction；
- Provider registry、Provider ID 或模型选择；
- base URL 默认值和 credential lookup；
- OAuth login、token persistence、refresh 或 revoke；
- API relative endpoint path；
- Provider request/response JSON；
- `data:` JSON 的 schema；
- Anthropic `ping`、OpenAI terminal event 或 `[DONE]` 的语义；
- prompt cache 参数；
- tool/reasoning/usage/stop reason 归一化；
- 是否 fallback model/provider；
- catalog refresh/merge；
- Agent Turn、Thread 或 durable state。

## 3. 与上下层的接口

`zeta-api` 构造 protocol request 并消费 operation result；`zeta-client` 只包装 operation policy：

```text
zeta-api::endpoint + zeta-api::requests
        │ encoded request + response decoder
        ▼
zeta-client
        │ HttpRequest
        ▼
zeta-http-client
        │ HttpResponse | raw byte stream
        ▼
zeta-client retry/framing
        │ response facts | SseFrame | NdjsonRecord
        ▼
zeta-api decoder
        │
        ▼
canonical ModelResponse | ModelStreamEvent
```

目标 public shape 可以是：

```rust
pub struct ClientOperation {
    pub request: zeta_http_client::HttpRequest,
    pub retry: RetryPolicy,
    pub deadline: OperationDeadline,
    pub telemetry: OperationTelemetry,
}

pub struct ClientOperationResponse {
    pub response: zeta_http_client::HttpResponse,
    pub attempts: AttemptSummary,
    pub timing: OperationTiming,
}
```

示例名称不要求逐字实现，但 `zeta-client` 不复制 URL/header/body、proxy、TLS 或 pool 类型，也不能
直接返回 `serde_json::Value`。JSON decoding 是 API 协议职责。

普通 OAuth exchange、catalog discovery、Plugin/MCP request 等不需要 operation retry/framing 的
调用方可以直接使用 `zeta-http-client`，不强制经过本 crate。

## 4. Retry

### 4.1 机制和策略分开

`zeta-client` 拥有 retry 机制，但不猜测 operation 是否可以重放：

```text
caller/API/runtime
  提供 RetryPolicy + RetryClassifier
        ↓
zeta-client
  分配 attempt budget
        ↓
zeta-http-client execute one attempt
        ↓
zeta-client classify → wait → next attempt
```

建议 typed safety：

```rust
pub enum RetrySafety {
    Never,
    Idempotent,
    ExplicitIdempotencyKey,
}

pub struct RetryPolicy {
    pub safety: RetrySafety,
    pub max_attempts: NonZeroU8,
    pub backoff: BackoffPolicy,
    pub retry_after: RetryAfterPolicy,
}
```

禁止使用 `retry: bool`。调用点必须清楚表达为什么允许重放。

认证恢复不属于普通 retry loop。收到 `401 Unauthorized` 时，client 返回 response facts；
`zeta-model-provider` 只可按 direct-provider credential 的正式语义执行一次受限 rebuild/refresh。
client 不读取 `zeta-secrets`、不持有 refresh token，也不根据 401 自行重试。ChatGPT/Codex
subscription refresh 归 upstream Codex App Server，Zeta 只接收稳定的 reauthentication outcome。

`zeta-client` 不提供带登录状态的 `OAuthClient` facade。未来一个 officially supported OAuth adapter
需要普通 HTTP execution 时，它可使用 `zeta-http-client`，但其生命周期属于 `zeta-login` 和 exact
provider adapter，而不是 operation retry/framing layer。

RFC 9110 指出，client 不应自动重试非幂等请求，除非它知道请求语义实际幂等或能确定原请求未被
应用。因此：

- inference POST 默认 `Never`；
- catalog GET 可以由其调用方选择 `Idempotent` operation policy；
- 显式 idempotency key 只有在 Provider 文档确认语义后才能启用；
- client 不根据“未收到 response byte”猜测 POST 安全；
- 一次 auth recovery 不能嵌套出第二套无限 retry；
- retry budget 同时受 max attempts 和 operation deadline 限制。

### 4.2 Attempt classifier

Classifier 可以使用：

- `zeta-http-client` 返回的 transport failure phase；
- 是否已发送完整 request body；
- 是否已收到 response headers/body/frame；
- HTTP status；
- `Retry-After`；
- 调用方从 bounded error body 得到的 typed classification；
- 是否已经向消费者发布 semantic output。

Client 不解析 Provider error JSON。需要 body-aware classification 时，`zeta-api` 提供受限的
classifier hook；hook 只能返回 retry decision 和低敏感 evidence，不把 raw body 写入 telemetry。

### 4.3 Backoff

- exponential backoff 必须 bounded；
- jitter 必须可测试并可注入；
- `Retry-After` 解析失败时使用本地 policy，不 panic；
- 超出 overall operation deadline 时不启动新 attempt；
- cancellation 立即中止 backoff 并传递给活跃 transport attempt；
- 测试使用 fake clock，不真实 sleep。

## 5. SSE 与 NDJSON

### 5.1 SSE framing

`zeta-client::stream::sse` 消费 `zeta-http-client` 的 raw byte stream，并按 WHATWG event stream
format 处理：

- UTF-8；
- CRLF、CR 和 LF；
- `event`、`data`、`id`、`retry` field；
- 多行 `data` 拼接；
- comment line；
- 空行 dispatch；
- 跨任意 byte chunk boundary；
- EOF 和尾部不完整 event；
- frame/buffer hard limit。

SSE 的 `retry:` field 可以作为 frame evidence 保留，但不能自动触发 inference stream 重连。
EventSource 的自动重连语义不能直接套用到可能计费、可能已经产生副作用的模型 POST；只有具体 API
文档定义 resumable stream，并由 runtime 显式选择策略时，client 才能重新连接。

输出是 Provider-neutral frame：

```rust
pub struct SseEvent {
    pub event: Option<String>,
    pub data: String,
    pub id: Option<String>,
}

pub enum SseFrame {
    Event(SseEvent),
    Comment,
}
```

具体 API decoder 决定：

- `event: ping` 是否只是 liveness；
- `data: [DONE]` 是否终止；
- `response.output_text.delta` 如何映射；
- 未知 event 是否可忽略；
- 何时形成 canonical completed。

### 5.2 NDJSON framing

NDJSON framer 只输出 bounded JSON record bytes，不解析 Ollama `done` 或 `error`：

```rust
pub struct NdjsonRecord {
    pub bytes: BoundedBytes,
}
```

空行、尾部不完整 JSON、超限 record 和 invalid UTF-8 形成 client framing error；record 中字段的
语义错误由 `zeta-api` 返回。

### 5.3 Liveness

`zeta-http-client` 维护 raw wire activity 和 transport idle timeout；`zeta-client` 维护：

- `last_frame_activity`；
- first frame time；
- operation 是否已经发布 frame。

Client 不维护“模型是否有语义进展”。Anthropic `ping`、comment heartbeat 和长 reasoning 都可能
维持连接但不产生 text。`last_semantic_progress` 由 API/runtime 在解码后维护。

## 6. Operation deadline、取消与 backpressure

Deadline 分层：

| Owner | Deadline |
| --- | --- |
| `zeta-http-client` | DNS/connect/TLS/first-byte/idle/single-attempt |
| `zeta-client` | 包含 retry/backoff 的 overall operation |
| runtime | Turn、auth recovery 或产品流程 deadline |

Operation client 在每次 attempt 前计算 remaining budget，并把 bounded attempt deadline 交给
`zeta-http-client`。任一上层 deadline 结束后不能启动新 attempt。

当前 unary 路径的 cancellation 从 runtime 贯穿 operation preflight、活跃 attempt 的本地等待和
retry timer。取消返回独立 `ClientError::Cancelled`，不会包装成 retryable network failure，也
不会启动下一次 attempt。由于 `zeta-http-client` 仍使用同步 `ureq`，已经进入 socket 的 attempt
不能被 token 强制关闭；它由 bounded transport timeout 收束，response 即使迟到也不再被接受。
未来 streaming 路径还需把 token 继续贯穿 raw byte stream、framer 和 bounded frame channel。

Raw stream 的 socket/buffer backpressure 属于 `zeta-http-client`；SSE/NDJSON framed channel 的
backpressure 属于 `zeta-client`。Client 不能合并或丢弃它不理解的 Provider payload。

## 7. Telemetry

Telemetry 分为两层：

| `zeta-http-client` | `zeta-client` |
| --- | --- |
| DNS/connect/TLS/HTTP timing | operation duration |
| proxy/redirect/pool evidence | attempt count |
| request/response byte count | retry reason/backoff |
| status class/transport error | first frame/frame count |
| transport timeout/cancel | framing/operation outcome |
| HTTP redaction policy | low-cardinality API metadata |

业务调用方只提供低基数 operation metadata，例如：

```text
operation.kind = model_inference | model_catalog
api.profile = openai_responses | anthropic_messages
provider.kind = openai | anthropic | custom
```

禁止把 exact model ID、URL、header value、credential、prompt、tool arguments/output、reasoning、
raw response body 或 stream payload 放进 operation log/label。底层完整规则以
[`zeta-http-client` README](../zeta-rs/http-client/README.md#telemetry) 为准。

## 8. Error

目标 operation error 只增加本层语义：

```rust
pub enum ClientError {
    Transport(zeta_http_client::HttpClientError),
    OperationDeadlineExceeded,
    Cancelled,
    Framing(StreamFramingError),
    RetryExhausted(RetryExhaustedError),
}
```

Provider error 不是 `ClientError`，由 `zeta-api` 解码。Error 可以包含 attempt、phase 和 bounded
timing evidence，但不能复制 transport secret 或 raw body。

## 9. 目标目录

```text
zeta-rs/zeta-client/
├── BUILD.bazel
├── Cargo.toml
└── src/
    ├── lib.rs
    ├── operation/
    │   ├── mod.rs
    │   ├── request.rs
    │   ├── response.rs
    │   ├── deadline.rs
    │   └── operation_tests.rs
    ├── retry/
    │   ├── mod.rs
    │   ├── policy.rs
    │   ├── classifier.rs
    │   ├── backoff.rs
    │   └── retry_tests.rs
    ├── stream/
    │   ├── mod.rs
    │   ├── sse.rs
    │   ├── ndjson.rs
    │   ├── activity.rs
    │   ├── limits.rs
    │   ├── sse_tests.rs
    │   └── ndjson_tests.rs
    ├── telemetry/
    │   ├── mod.rs
    │   ├── operation.rs
    │   ├── attempt.rs
    │   ├── stream.rs
    │   └── telemetry_tests.rs
    └── error.rs
```

Backend、proxy、TLS、redirect、raw HTTP types 和 pool implementation 都不出现在该目录；它们属于
`zeta-http-client`。

## 10. Public API

长期 public API 只导出：

- `ZetaClient` 或等价 operation executor；
- `ClientOperation` / `ClientOperationResponse`；
- `SseFrame` / `SseEvent`；
- `NdjsonRecord`；
- `RetryPolicy` / `RetrySafety`；
- operation deadline/cancellation value；
- `ClientError` 和安全 attempt evidence；
- 必要的 operation telemetry context。

HTTP request/response/client/config/error 来自 `zeta-http-client`，不在本 crate 建立平行 value。
新增 public trait 必须说明实现者对 retry、operation deadline、framing、redaction 和 backpressure
的责任。

## 11. 测试

- retry max attempts、jitter、`Retry-After` 和 overall budget；
- non-idempotent request 默认不 replay；
- cancellation during backoff/attempt/stream；
- transport failure phase 到 classifier 的映射；
- 任意 byte fragmentation 与 UTF-8 split；
- SSE multiline data、comment、CRLF/LF 和 EOF；
- NDJSON blank line、oversized/incomplete record；
- framed channel backpressure；
- operation telemetry 低基数和 secret negative tests；
- fake clock/fake `zeta-http-client`，不访问真实 Provider。

Proxy、TLS、redirect、HTTP timeout 和 pool tests 只属于 `zeta-http-client`，不在本 crate 重复。

## 12. 迁移顺序

1. 建立 `zeta-http-client` 的 raw HTTP value、client/config/error port。
2. 将 `HttpHeader`、raw request/response、`HttpClient` 和 `UreqHttpClient` 从本 crate 迁出。
3. 本 crate 改为依赖 `zeta-http-client` 并保留现有 unary behavior。
4. 把 operation retry loop 改为执行一个或多个 bounded transport attempts。
5. 增加 live raw stream execution 后接入现有 SSE framer。
6. 增加 NDJSON framer、operation deadline 与 framed backpressure。
7. 拆分 transport telemetry 与 operation telemetry。
8. 更新 `zeta-api` 和 `model-provider`，删除旧 raw transport compatibility surface。

当前处于开发阶段，迁移时直接更新全部调用方，不建立旧 `UreqHttpClient` public API 的长期兼容层。

## 13. 固定决策

1. `zeta-client` 是 API operation client，不是 workspace HTTP backend。
2. `zeta-http-client` 独占 proxy、TLS、redirect、attempt timeout、pool 和 transport redaction。
3. `zeta-client` 拥有 retry 机制，调用方提供 typed retry safety/policy。
4. `zeta-client` 拥有 SSE/NDJSON framing，不解释 Provider event。
5. Inference POST 默认不透明 retry。
6. Wire activity、frame activity 与 semantic progress 分开。
7. Client 不依赖 Provider registry、config、Core 或 App Server。
