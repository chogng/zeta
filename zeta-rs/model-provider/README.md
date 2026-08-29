# `zeta-model-provider`

> 本 README 解释 provider runtime instantiation、adapter selection 与 immutable model invoker。
> Declarative config 见 [`zeta-model-provider-config`](../model-provider-config/README.md)，跨系统
> credential/provider 设计见 [`docs/model-provider.md`](../../docs/model-provider.md)，模型目录实现见
> [`zeta-models-manager`](../models-manager/README.md)。

`zeta-model-provider` 把 validated declarative config 与 `ModelRef(provider, model)` 解析成
`Arc<dyn ModelInvoker>`。它选择 provider runtime 和 API profile；wire codec 属于 `zeta-api`，
operation retry/framing 属于 `zeta-client`，HTTP transport 与共享 network policy 属于
`zeta-http-client`。WebSocket handshake/message transport 已独立位于 `zeta-websocket-client`；本 crate 尚未组合
Responses WebSocket codec 或 `ModelClientSession`。

当前 `EmbeddingInvoker` / `RerankInvoker` 除 canonical、有序、provider-neutral 调用契约外，已接入
OpenAI-compatible embedding/rerank wire codec、OpenAI/Ollama embedding runtime 和 exact provider
config resolver。本 crate 仍只拥有模型 API 选择、credential materialization、请求适配和执行。
它不决定代码如何切块、查询哪个向量索引、准备哪些 rerank 候选，也不拥有排序、过滤或截断
策略；本地 Codebase 的这些策略属于 `zeta-codebase`，远端托管索引则属于具体 provider。

## 公共契约

| Symbol | 职责 | 生命周期 |
| --- | --- | --- |
| `ModelProvider` | `ModelRuntimeRequest → ModelInvoker` port | composition/invocation safe point |
| `ModelProviderRuntime` | built-in concrete resolver | 持有 config registry、shared `ModelsManager`、lazy operation client 和可选 local tokenizer service |
| `ModelRuntimeRequest` | exact `ModelRef + ModelProviderConfig` | immutable selection request |
| `ModelInvoker` | canonical `ModelRequest → ModelResponse` | one immutable provider/model snapshot |
| `ModelInvoker::{input_token_measurement_capability,measure_input_with_cancellation}` | frozen request 的 tokenizer/preflight port | 与 invocation 相同 immutable snapshot |
| re-exported `LocalTokenizerBinding` / `LocalTokenizerRegistry` | 宿主安装资产后的通用本地 tokenizer 接口 | composition safe point 构建后注入 runtime |
| `EmbeddingInvoker` | ordered text batch → finite equal-dimension vectors | one immutable embedding model snapshot |
| `RerankInvoker` | query + ordered documents → ordered finite scores | one immutable rerank model snapshot |
| `SemanticModelProvider` | exact model/config → embedding 或 rerank invoker | provider transport/credential boundary |
| `Provider` | normalized provider runtime | definition、config、private adapter、client |
| `UnavailableModel` | explicit failing invoker | host 无法配置 model 时 fail closed |
| `EchoModel` | deterministic test/local fixture | 不是 production model |
| `ModelProviderError` | config/model/API/credential/unavailable error | 保留 failure domain |

App Server 应在每次 model invocation safe point 重新 resolve invoker；config update 影响下一次
invocation，不原地修改已经运行的 `RegisteredModelInvoker`。

## 内部接口地图

| Symbol | 可见性 | 当前职责 | 方向约束 |
| --- | --- | --- | --- |
| `ModelProviderRuntime::instantiate_normalized` | private method | definition lookup + `Provider::instantiate` | normalization success 后 provider 必须存在 |
| `LazyOperationClient` | private struct | 第一次 operation 才创建 production HTTP client，并缓存结果 | App Server 启动和 config inspection 不接触 TLS/proxy |
| `Provider::instantiate` | crate-private | enforce definition/config ID equality，materialize adapter | 不读取 mutable config/credential store |
| `providers::instantiate` | crate-private function | exhaustive `ProviderAdapter` enum dispatch | provider selection 唯一 switch |
| `ProviderAdapter` | crate-private trait | protocol + explicit token measurement capability + complete | 不按 model ID 或 URL 猜能力 |
| `LocalInputTokenCounter` | crate-private struct | 官方预检不可用时把整份请求交给本地计数服务 | 不下载资产、不按 provider 猜 tokenizer revision |
| `api_endpoint` | private function | `ApiProfile → zeta_api::ApiEndpoint` | 按 profile，不按 provider name 猜 |
| provider `*Adapter::new` | crate-private | normalized base URL + fixed headers + endpoint | one immutable runtime snapshot |
| `Provider::resolve_model` | private method | 委托 shared manager 的 static typed resolution | 不复制 catalog policy 或做远端请求 |
| `RegisteredModelInvoker` | private struct | bind exact Provider + resolved Model | request 时只应用 normalized defaults |
| `RegisteredModelInvoker::invoke` | private trait impl | clone canonical request、apply max tokens、complete | 不读取 product config |

## 运行时调用图

```text
ModelProviderRuntime::runtime(ModelRuntimeRequest)
└─ build_model(config, model_ref)
   ├─ ProviderConfigRegistry::normalize_for
   ├─ instantiate_normalized
   │  ├─ registry.get(definition)
   │  └─ Provider::instantiate
   │     └─ providers::instantiate(adapter kind, normalized config)
   └─ Provider::build_model(model_id)
      ├─ Provider::resolve_model
      │  └─ ModelsManager::resolve_static
      └─ RegisteredModelInvoker { provider, model }

RegisteredModelInvoker::invoke(request)
├─ clone canonical ModelRequest
├─ apply normalized max_output_tokens when the request has no explicit limit
└─ Provider::complete
   ├─ resolve_model
   └─ ProviderAdapter::complete
      └─ zeta_api::ApiEndpoint::complete_with_client
         └─ LazyOperationClient
            ├─ first operation: build fallible production client
            └─ OperationClient
```

`RegisteredModelInvoker::measure_input_with_cancellation` 复用同一个 `prepare_request`，因此 provider
默认 `max_output_tokens` 与最终 invocation 一致；adapter 再把 canonical input 编成 count endpoint
接受的 wire shape。`ProviderInputTokenCounter` 只 materialize config 已声明且 model policy 允许的
profile/target，adapter 不按 model ID 或 URL 猜能力。OpenAI Responses 声明 remote/exact；Anthropic、
Google、Kimi 与 Z.AI 声明 remote/estimated。`Provider` 统一执行“官方预检 → 本地整请求计数 →
unavailable”的优先级；因此任何 provider 只要注入精确 `ModelRef` binding 都能使用 local/estimated，
不再由各 adapter 复制本地选择逻辑。

`Provider::complete` 再次通过同一个 manager resolve model，因此 direct Provider callers 与 bound
invoker 使用相同 catalog policy。App Server 通过 `ModelProviderRuntime::models_manager()` 获得同一
进程内 manager clone，目录展示与 invocation 不再分别读取 `ProviderDefinition.models`。

## 供应商适配器模式

每个 `src/providers/<name>.rs` 定义 private adapter，通常持有：

- `ResolvedApiTarget`：normalized base URL 与 fixed headers；
- `ApiEndpoint`：由 declarative `ApiProfile` 映射；
- 必要 provider-fixed header。

Adapter 不手写 request JSON；它委托 `zeta-api` codec。相同 provider adapter 可以选择不同
profile，但只有 configuration definition 可以决定 profile。

新增 provider/profile 时同步检查 config enum/definition、`providers::instantiate` exhaustive match、
`api_endpoint`、fixed headers、codec support、fake transport tests 和系统 provider matrix。

## 错误与模型目录

| Path | Error |
| --- | --- |
| invalid/unknown/mismatched config | `ModelProviderError::Config` |
| listed-only unknown model | `ModelNotRegistered` |
| 供应商上下文上限 | `ContextOverflow` |
| 供应商认证或授权失败 | `AuthFailed` |
| 无效 canonical/供应商请求 | `InvalidRequest` |
| 无效供应商响应 | `InvalidResponse` |
| 传输、限流、过载或其他 HTTP 状态 | `Api(ApiError)` |
| explicitly unavailable host model | `Unavailable` |

`From<ApiError>` 将四个语义类别提升为直接的 `ModelProviderError` variant，其余操作类别保留在 `Api` 中。原始供应商详情只供产品宿主的受控日志使用；跨入 Core 时必须改成无原文的类型化 `CoreError`。

`AllowUnlisted` 由 manager 生成 unverified metadata，不证明 entitlement、capability 或 remote
availability。`ListedOnly` 由 manager 在任何 network call 前拒绝。

## 方向偏差检查

- Core 按 provider ID branch：provider selection 泄漏出 runtime crate；
- runtime 根据 URL 猜 profile：declarative config 被绕过；
- provider adapter 自己序列化 wire JSON：codec ownership 从 `zeta-api` 漂移；
- adapter 直接构造 ureq/reqwest client：network substrate/retry ownership 漂移；
- `RegisteredModelInvoker` 读取 mutable config：safe-point snapshot 被破坏；
- `EchoModel` 出现在 production fallback：配置失败被静默掩盖；
- API codec/network layer 读取 secret store：credential materialization 必须停留在 provider runtime。

## 测试、限制与演进

```text
cargo test -p zeta-model-provider
bazel test //zeta-rs/model-provider:model-provider-unit-tests
```

测试使用注入的 `OperationClient` 捕获请求，覆盖 Responses/Chat/Anthropic 配置、结构化工具、
自定义端点、默认值、供应商不匹配、目录策略、固定标头、取消传播和默认 HTTP 传输；lazy client
测试另外断言构造前不访问 transport、只初始化一次，并把初始化失败作为普通 transport error 返回。

当前 completion `ModelInvoker` 已有 concrete provider adapters；embedding/rerank 已有 canonical
invoker、request/response validation、OpenAI-compatible/Ollama runtime resolver，以及本地
`zeta-codebase` 和 Tool Search consumers。
当前 completion invocation 同时支持 unary 与 OpenAI Responses、OpenAI-compatible Chat Completions、
Google Chat-compatible、Anthropic Messages 原生 HTTP/SSE stream。每个 immutable provider definition
显式发布 `ModelOutputTransport::{NativeStreaming, Unary}`；catalog/Desktop 只消费该声明，不从
provider 名称或 `ApiProfile` 猜测；
WebSocket eligibility 由独立的 `WebSocketApiProfile` fail closed 声明，不能从
`ModelOutputTransport` 或 HTTP compatibility 推断。底层 connector 已实现，但 protocol codec、session
reuse、sticky turn state、prewarm、`previous_response_id` 和 HTTP fallback 尚未进入本 runtime。
`ProviderCredentialService` 是供应商 API Key 的唯一所有者：App Server 通过它校验并写入 host 注入的 `SecretStore`，direct 和 semantic runtime 通过它解析 `ApiKeyPolicy` 与 `ApiKeyHeader`。`Provider` 合并 adapter 声明的固定 Header 与认证 Header，并唯一持有最终 `ResolvedApiTarget`；各 provider adapter 只负责协议、endpoint、固定 Header 和响应处理。Anthropic 使用 `x-api-key`、Google 使用 `x-goog-api-key`，其余远端 adapter 使用 Bearer Header；Ollama 不读取 Key，OpenAI-compatible 允许无 Key endpoint。
更多 stream profile 与动态 catalog 的长期设计仍在系统文档中演进。完整
ChatGPT subscription 通过 `zeta-chatgpt` 提供的 fresh authenticated target 进入 OpenAI Responses adapter；Agent loop 仍由 Zeta Core `TurnExecutor` 持有。
新增能力应保持 invoker immutable、profile explicit、
provider adapter private，以及 config/catalog/codec/operation/network 分层。

当前 runtime 尚未实现 provider-specific `ModelCatalogSource`；动态 discovery endpoint/DTO 和 credential
binding 属于下一阶段 adapter 工作。静态 invocation resolution 已统一进入 manager。

当前 input-token preflight 已贯穿 `ModelInvoker → ProviderAdapter → zeta-api → OperationClient` 的
caller-owned cancellation。所有 estimated endpoint 使用额外 1%/至少 32 tokens 的保守记账余量；这
只是 Zeta 的预算策略，不是 provider 承诺的硬上界。Google 对不能无损映射到 native request 的远程
图片返回 unavailable；Kimi 当前带 tools/reasoning 的请求返回 unavailable；Z.AI 当前带 Tool
Call/Result 历史的请求返回 unavailable。DeepSeek/Hugging Face 的本地 tokenizer adapter 已实现，
使用 2%/至少 64 tokens 的保守余量；宿主通过 `ModelProviderRuntime::with_local_tokenizers` 注入已经
验证的 registry 或 manager。App Server 当前为 Hugging Face 公共 `owner/repo` 注入按需发现、下载、
磁盘缓存和内存 LRU；其他 provider/model 仍需宿主提供固定资产清单。官方预检或本地计数的非取消
错误不会中止真实模型调用，而是继续降级到下一计量来源。

`RegisteredModelInvoker::invoke_with_cancellation` 把 caller token 逐层传给 private
`ProviderAdapter::complete` 和 `OperationClient::execute_with_cancellation`。取消是独立的
`ModelProviderError::Cancelled`，App Server 不会把它误报为 model failure。同步 HTTP attempt
可能已进入底层 socket；operation 立即停止等待且不 retry，attempt 本身由 transport timeout
有界结束，迟到 response 不会被接受。
