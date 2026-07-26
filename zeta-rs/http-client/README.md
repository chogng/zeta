# zeta-http-client (`zeta_http_client`)

> - 目标物理位置：`zeta-rs/http-client/`
> - Rust crate：`zeta_http_client`
> - 层次：Zeta 共享的 outbound network substrate
> - 当前状态：已实现同步 unary HTTP transport、DER CA bundle 与 mTLS；stream/WebSocket、cancellation 与完整 diagnostics 仍在后续范围
> - 上层 operation client：[`docs/zeta-client.md`](../../docs/zeta-client.md)

## 1. 结论

`zeta-http-client` 是 Zeta workspace 唯一的通用出站 HTTP/WebSocket 传输层。所有需要访问远程
服务的 Rust crate 都使用这里导出的 request、response、stream、error、configuration 和 client
类型，不得自行构造 `reqwest::Client`、`ureq::Agent`、`hyper` connector 或其他后端 client。

本 crate 统一解决：

- HTTP backend 创建与连接复用；
- environment、explicit 和 bypass proxy；
- 系统根证书、额外 CA、client identity 与 TLS validation；
- 安全、bounded 的 redirect；
- connect、TLS、first-byte、idle 和 attempt deadline；
- cancellation、response limit 与 raw byte-stream backpressure；
- HTTP/WebSocket transport logging、trace、metrics 与 secret redaction；
- provider-neutral network error 和 timing evidence。

它不解释 OpenAI、ChatGPT、Codex、Anthropic 或任何 Provider 协议。产品和协议名称只会出现在验收
场景中，不会进入公共 transport API。

一句话边界：

```text
zeta-http-client 负责“如何按照统一网络策略安全地传输 bytes”
zeta-client      负责“如何执行一个 Zeta API operation”
zeta-api         负责“这些 bytes 在具体 API 中表示什么”
```

## 当前实现范围

当前 crate 已提供可复用的 `UreqHttpClient` 和稳定的 raw HTTP port：

- `HttpRequest`、`HttpResponse`、`HttpHeader`、`HttpClientError` 与 `HttpClient`；
- 一个 client/agent 对应一份不可变 `HttpClientConfig`，连接池由该 agent 复用；
- `ProxyPolicy::{Direct, FromEnvironment, Explicit, ExplicitWithBypass}`；环境 proxy 与 `NO_PROXY`
  在 client 构造时快照解析，explicit proxy 的 debug 输出会脱敏；
- host system-root TLS verification，以及 `SystemPlus`/`CustomOnly` 的 DER CA bundle；
- mTLS client certificate chain 与 PKCS#1/PKCS#8/SEC1 DER private key，private key 释放时清零；
- 默认拒绝 redirect；只有显式 `RedirectPolicy::Follow { max_hops }` 才允许 bounded follow；
- connect、read、write 和 overall attempt timeout；
- 可配置的总 idle connection 与 per-host idle connection 限制；
- 10 MiB 默认、可配置的 unary response body hard limit；
- 不包含 URL、headers、证书或 payload 的低基数 transport telemetry hook；
- one-attempt execution：非 2xx response 会原样返回，transport 层绝不重试。

`zeta-client::ZetaClient` 包装 `Arc<dyn zeta_http_client::HttpClient>`，按照
`RetryPolicy` 在 operation 层执行重试；SSE framing 和 operation telemetry 也继续归该层。
response streaming、WebSocket、per-phase diagnostics 与 cancellation 是本设计文档定义的后续 contract，
尚未以半成品 API 暴露。当前 telemetry 只提供 attempt 的 method、status class、字节数和 elapsed time；
PEM 文件解析归 config/secrets 层；本 crate 接收已经解析的 DER bytes，避免把 filesystem 和
secret-resolution authority 带入 transport。

## 2. Workspace 依赖规则

```text
model-provider auth / catalog / plugin / MCP / updater
                         │ raw HTTP operation
                         ▼
                  zeta-http-client

model-provider ──▶ zeta-api ──▶ zeta-client ──▶ zeta-http-client
                    codec       retry/framing     socket/backend
```

规则如下：

1. 只有 `zeta-http-client` 可以直接依赖通用 HTTP backend。
2. Application composition root 调用本 crate 的 factory 创建共享 client，但不接触 backend 类型。
3. 上层通过 `Arc<dyn HttpClient>` 或拥有同等共享语义的 public handle 注入 client。
4. 禁止在 request path 中临时创建 client；这会分裂配置并破坏连接复用。
5. 测试通过同一 public port 注入 fake transport，不启动真实 Provider 请求。
6. 本 crate 接收已经解析完成的 endpoint、proxy credential、CA 和 client identity，不读取
   `zeta-config` 或 `zeta-secrets`。

`zeta-http-client` 可以被 `zeta-client`、auth、catalog、MCP 等多个上层直接使用。只有需要模型
operation retry 或 SSE/NDJSON framing 的调用才经过 `zeta-client`；普通 token exchange、discovery
或 webhook 请求不应被强制伪装成模型 operation。

## 3. 目标：Client 配置与生命周期

Network configuration 是一个不可变 snapshot。建议 public shape 使用带名称的 policy，而不是 bool
或含义不明的 `Option`：

```rust
pub struct HttpClientConfig {
    pub proxy: ProxyPolicy,
    pub tls: TlsPolicy,
    pub redirects: RedirectPolicy,
    pub timeouts: TransportTimeoutPolicy,
    pub pool: ConnectionPoolPolicy,
    pub diagnostics: NetworkDiagnosticsPolicy,
}

pub enum ProxyPolicy {
    Disabled,
    FromEnvironment,
    Explicit(ProxyEndpoint),
}

pub enum RedirectPolicy {
    Reject,
    Follow(RedirectLimits),
}
```

示例名称不是必须逐字实现，但调用点必须可以读出策略，不能出现：

```rust
HttpClient::new(true, None, false)
```

Composition root 在启动时完成 config resolution：

```text
config snapshot + resolved secrets + host defaults
        ↓
HttpClientFactory::build(...)
        ↓
Arc<dyn HttpClient>
        ↓
all outbound consumers
```

配置变化创建新的 client generation。新请求切换到新 generation，旧 client 只为已有 request/stream
继续存活并在完成后释放；不能原地修改一个正在被并发请求使用的 TLS/proxy/pool 配置。

## 4. 目标：Proxy

### 4.1 支持的来源

Proxy policy 至少区分：

- `Disabled`：明确不使用 proxy；
- `FromEnvironment`：读取 host 解析后的 `HTTP_PROXY`、`HTTPS_PROXY`、`ALL_PROXY` 与
  `NO_PROXY` snapshot；
- `Explicit`：使用配置指定的 proxy endpoint 和已经解析的 credential。

环境变量只在构造 client 时解析一次，不能在每个请求中读取进程环境。Config authority 可以在配置
变化后创建新 generation。

`NO_PROXY` 的 hostname、domain suffix、IP literal、port 和 wildcard 行为必须有 contract tests。
暂不支持的 SOCKS、PAC、system proxy discovery 或 proxy auto-auth 必须在构造阶段返回明确的
configuration error，不能静默直连。

### 4.2 Proxy 安全边界

- Proxy credential 使用默认 redacted、drop-safe 的 sensitive value；
- proxy URL、username、password 和 `Proxy-Authorization` 不写入 error、log 或 metric；
- HTTPS target 通过 proxy 时使用 CONNECT tunnel，不把 origin credential 发给 proxy；
- proxy failure 与 origin failure 使用不同 error phase；
- bypass 决策在 DNS 前由规范化 target authority 完成，并保留低敏感 diagnostic evidence；
- explicit proxy 不因环境变量变化而被覆盖。

## 5. 目标：TLS 与证书

TLS policy 至少表达：

```rust
pub enum TrustRoots {
    System,
    SystemPlus(AdditionalCaBundle),
    CustomOnly(CustomCaBundle),
}

pub enum ClientIdentityPolicy {
    None,
    Identity(ClientIdentity),
}
```

固定安全规则：

- 默认启用证书链、有效期与 hostname verification；
- 不提供 `danger_accept_invalid_certs: bool` 一类容易误用的 public escape hatch；
- 企业 TLS interception 和 private root CA 通过显式 additional CA 支持；
- PEM/DER parse、empty bundle、expired certificate 和 key mismatch 在 client 构造阶段失败；
- CA、private key 和 client certificate 原文不得进入 `Debug`、error、trace 或 panic message；
- TLS config 属于 client generation，变更后创建新连接池；
- TLS version、ALPN 和 HTTP version negotiation 的结果只作为低敏感 transport evidence。

如果开发环境未来确实需要不安全 TLS，必须通过单独的高摩擦 host-only capability，并确保 release
composition 无法启用；它不能成为普通 crate API。

## 6. 目标：Redirect

Redirect 默认 `Reject`。显式启用时必须 bounded，并定义：

- 最大 hop 数与 loop detection；
- 是否允许 scheme downgrade；默认拒绝 HTTPS → HTTP；
- same-origin 与 cross-origin 的 header forwarding；
- method/body 是否可以 replay；
- redirect chain 如何计入 overall attempt deadline；
- response body 和 location 长度上限。

跨 origin 时必须移除：

- `Authorization`；
- cookie；
- proxy credential；
- Provider-specific credential header；
- 调用方标记为 sensitive 或 origin-bound 的 header。

Transport 可以识别 HTTP redirect facts，但不能猜测某个 OAuth、模型 POST 或 upload operation 是否
可以改变 method 或重放 body。默认只自动跟随调用方明确允许的 safe request；其余 redirect 作为
response facts 返回上层。

浏览器 OAuth authorization redirect、loopback callback listener 和 state validation 不属于这里。
本 crate 只负责 token exchange 等实际 HTTP request 的 transport redirect policy。

## 7. 目标：Timeout、deadline 与 cancellation

本 crate 负责单次 transport attempt 的 deadline：

| Phase | 含义 |
| --- | --- |
| DNS | 名称解析 |
| Connect | TCP 或 proxy tunnel 建立 |
| TLS | TLS handshake |
| First byte | request 发出到第一个 response byte |
| Idle | 已收到 response 后没有 wire activity |
| Attempt | 一个 HTTP attempt 的总上限 |

`zeta-client` 负责包含 retry/backoff 在内的 operation deadline。每次 attempt 获得 operation
remaining budget 的子 deadline，不能超过剩余预算。

Cancellation 是独立终态，不折叠成普通 timeout 或 connection reset。取消必须中止或释放：

- DNS/connect/TLS future；
- request body producer；
- response body reader；
- bounded stream channel wait；
- WebSocket handshake 和活跃 socket；
- 当前 transport span。

Raw response body 与 stream buffer 都必须有 hard limit。Unary response 超限立即失败；stream
使用 bounded buffer 和 backpressure，不能因为消费者变慢而无限积累内存。

## 8. 目标：连接复用

共享 client 的主要目的之一是拥有稳定连接池。Pool key 至少受以下因素隔离：

- scheme、host、port；
- proxy route；
- TLS trust/client identity generation；
- negotiated HTTP version；
- backend 明确要求的 connection affinity。

规则如下：

- process/host scope 通常只创建一个 config generation 的 client；
- keep-alive、idle eviction、per-host concurrency 与 pool upper bound 使用 typed policy；
- 配置或 credential boundary 不兼容的请求不能复用同一连接；
- request 完成前 response body 必须被 drain 或显式取消，避免污染连接状态；
- WebSocket session reuse 由上层提供 session/affinity intent，底层只管理 socket 生命周期；
- pool 状态不能暴露 credential、tenant 或完整 origin 到通用 metrics。

Backend 可以使用 `reqwest`/`hyper` 的 pool，但这些类型不出现在 public API。替换 backend 不应改变
上层的 request construction、timeout、error 或 telemetry contract。

## 9. 目标：Logging、trace 与 metrics

本 crate 拥有 transport-level diagnostics：

- sanitized scheme/authority 和 route template；
- network phase、HTTP version、status class；
- DNS/connect/TLS/first-byte/total timing；
- request/response byte count；
- redirect hop、proxy route kind、pool reuse outcome；
- timeout/cancellation/error classification。

默认禁止记录或作为 label：

- URL query、fragment 或含 ID/secret 的完整 path；
- authorization、cookie、proxy authorization 或任意 header value；
- CA、private key、token、API key、account 或 tenant identity；
- request/response body；
- SSE、NDJSON 或 WebSocket payload；
- exact model ID 或 prompt/tool content。

调用方只提供 static、低基数的 `NetworkOperation`，例如 `model_inference`、`oauth_exchange` 或
`model_catalog`。`zeta-client` 可以在其 operation span 中记录 retry/framing facts，但不能绕开本
crate 的 HTTP redaction policy。

所有 error 的 `Display` 和 `Debug` 都必须通过 secret negative tests。临时 wire dump 属于显式、
短期、host-controlled 的诊断能力，默认关闭且不得在 production build 中意外启用。

## 10. 目标：HTTP 与 WebSocket

HTTP public port 返回 provider-neutral facts：

```rust
pub trait HttpClient: Send + Sync {
    fn execute(&self, request: HttpRequest) -> HttpResponseFuture;
    fn open_stream(&self, request: HttpRequest) -> HttpStreamFuture;
    fn connect_websocket(&self, request: WebSocketRequest) -> WebSocketFuture;
}
```

实际 async shape 以后续 runtime 决策为准。新增 public trait 必须用 doc comment 说明实现者如何满足
proxy、TLS、redirect、deadline、cancellation、limit、redaction 与 connection reuse contract。

HTTP status（包括 3xx、4xx、5xx）是 response facts，不自动成为 `HttpClientError`。
`HttpClientError` 只表达请求无效、配置、DNS/connect/proxy/TLS、deadline、cancellation、body
limit、HTTP framing 或 socket failure。

本 crate 处理 HTTP/WebSocket wire framing，但不处理：

- SSE field 与 event dispatch；
- NDJSON record；
- Provider JSON；
- `[DONE]`、heartbeat 或 terminal event；
- ChatGPT/Codex session、conversation 或 model semantics。

这些 operation/protocol framing 属于 `zeta-client` 或 `zeta-api`。

## 11. ChatGPT、Codex 与其他验收场景

ChatGPT/Codex 是关键 consumer，但不是 transport type。至少验证：

1. API-key Provider、ChatGPT/Codex login 和 WebSocket 共用一致的 proxy/CA policy。
2. 企业 proxy 和 additional CA 同时覆盖 HTTPS、OAuth exchange 与 WebSocket handshake。
3. ChatGPT/Codex access token、cookie 和 session-affinity header 不出现在日志中。
4. Cross-origin redirect 不转发 authorization 或 origin-bound header。
5. Token refresh request 的 HTTP failure 原样返回 auth 层，不在 transport 内自行登录或刷新。
6. SSE/WebSocket cancellation 能及时释放 socket，operation 层可以区分取消与网络失败。

同样的底层能力必须服务 OpenAI API、Anthropic、Google、custom Provider、catalog、MCP/Plugin
remote transport 等调用方，不能为 ChatGPT/Codex 创建平行 backend。

## 12. 与 `zeta-client` 的边界

| 能力 | `zeta-http-client` | `zeta-client` |
| --- | --- | --- |
| backend、socket、pool | 拥有 | 使用 |
| proxy、TLS、redirect | 拥有 | 不覆盖 |
| attempt timeout/cancel | 执行 | 分配 operation budget |
| raw HTTP body/byte stream | 拥有 | 消费 |
| HTTP telemetry/redaction | 拥有 | 添加 operation metadata |
| retry safety/classifier/backoff | 提供 attempt facts | 拥有 |
| SSE/NDJSON framing | 不拥有 | 拥有 |
| Provider event/JSON | 不拥有 | 不拥有，由 `zeta-api` 解释 |

`zeta-client` 不得 re-export backend type，也不得创建另一份 proxy/TLS/pool configuration。若上层
只需要普通 HTTP，则直接依赖 `zeta-http-client`；若需要 Zeta model operation、retry 或 stream
framing，则使用 `zeta-client`。

## 13. 测试

测试使用 local fake server、fake DNS/clock、test certificate 和 fake transport，不访问真实
Provider：

- proxy env precedence、`NO_PROXY`、CONNECT、auth redaction 和 unsupported scheme；
- system/additional/custom CA、hostname mismatch、expired cert、mTLS identity；
- redirect hop/loop/downgrade、cross-origin header stripping 和 body replay rejection；
- DNS/connect/TLS/first-byte/idle/attempt deadline；
- cancellation during every network phase and bounded stream wait；
- unary/stream/header/redirect limit；
- pool reuse、isolation、idle eviction、config generation rollover；
- HTTP/1.1、HTTP/2 与 WebSocket handshake；
- error/log/metric secret negative tests；
- ChatGPT/Codex login/API/WebSocket 形态的 local contract fixtures。

## 14. 实现与迁移顺序

1. 创建 `zeta-http-client` Cargo/Bazel crate 和 provider-neutral value/error types。
2. 引入不可变 `HttpClientConfig`、factory 与共享 client handle。
3. 实现 proxy、TLS、redirect、timeout、limit 和 safe diagnostics contract tests。
4. 将当前 `zeta-client::UreqHttpClient` 与 raw request/response port 迁入本 crate。
5. 选择 production backend；backend module 保持 private。
6. 将所有直接 HTTP caller 迁移到共享 client，禁止 workspace 其他 crate 直接依赖 backend。
7. `zeta-client` 保留 operation retry、SSE/NDJSON framing 和 operation telemetry。
8. 增加 WebSocket port，并用 ChatGPT/Codex 与 local fixture 验证统一网络 policy。

当前处于开发阶段，不为临时 `zeta-client::HttpClient` 建立长期兼容层；迁移时一次性更新全部调用方。

## 15. 固定决策

1. `zeta-http-client` 是 workspace 唯一的通用 outbound network substrate。
2. HTTP backend 与其 builder/client/agent 类型保持 private。
3. 上层只使用本 crate 的 typed request/response/config/client。
4. Proxy、TLS、redirect、attempt timeout、pool 和 transport redaction 只在本 crate 实现。
5. `zeta-client` 拥有 operation retry 与 SSE/NDJSON framing。
6. ChatGPT/Codex 是验收场景，不进入 transport public API。
7. Config snapshot 不可变；变化通过 client generation rollover 生效。
8. 默认安全策略不能通过 bool 或普通 `Option` 被无意关闭。
