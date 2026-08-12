# `zeta-model-provider`

> 本 README 解释 provider runtime instantiation、adapter selection 与 immutable model invoker。
> Declarative config 见 [`zeta-model-provider-config`](../model-provider-config/README.md)，跨系统
> credential/provider 设计见 [`docs/model-provider.md`](../../docs/model-provider.md)。

`zeta-model-provider` 把 validated declarative config 与 `ModelRef(provider, model)` 解析成
`Arc<dyn ModelInvoker>`。它选择 provider runtime 和 API profile；wire codec 属于 `zeta-api`，
operation retry/framing 属于 `zeta-client`，socket/TLS/proxy 属于 `zeta-http-client`。

当前 `EmbeddingInvoker` / `RerankInvoker` 已定义 canonical、有序、provider-neutral 调用契约；
concrete provider codec 和 runtime resolver 尚未接入。本 crate 仍只拥有模型 API 选择、请求适配和执行。
它不决定代码如何切块、查询哪个向量索引、准备哪些 rerank 候选，也不拥有排序、过滤或截断
策略；这些属于调用模型的 CodeIndex 服务。

## 公共契约

| Symbol | 职责 | 生命周期 |
| --- | --- | --- |
| `ModelProvider` | `ModelRuntimeRequest → ModelInvoker` port | composition/invocation safe point |
| `ModelProviderRuntime` | built-in concrete resolver | 持有 config registry + shared operation client |
| `ModelRuntimeRequest` | exact `ModelRef + ModelProviderConfig` | immutable selection request |
| `ModelInvoker` | canonical `ModelRequest → ModelResponse` | one immutable provider/model snapshot |
| `EmbeddingInvoker` | ordered text batch → finite equal-dimension vectors | one immutable embedding model snapshot |
| `RerankInvoker` | query + ordered documents → ordered finite scores | one immutable rerank model snapshot |
| `Provider` | normalized provider runtime | definition、config、private adapter、client |
| `UnavailableModel` | explicit failing invoker | host 无法配置 model 时 fail closed |
| `EchoModel` | deterministic test/local fixture | 不是 production model |
| `ModelProviderError` | config/model/API/unavailable error | 保留 failure domain |

App Server 应在每次 model invocation safe point 重新 resolve invoker；config update 影响下一次
invocation，不原地修改已经运行的 `RegisteredModelInvoker`。

## 内部接口地图

| Symbol | 可见性 | 当前职责 | 方向约束 |
| --- | --- | --- | --- |
| `ModelProviderRuntime::instantiate_normalized` | private method | definition lookup + `Provider::instantiate` | normalization success 后 provider 必须存在 |
| `Provider::instantiate` | crate-private | enforce definition/config ID equality，materialize adapter | 不读取 mutable config/credential store |
| `providers::instantiate` | crate-private function | exhaustive `ProviderAdapter` enum dispatch | provider selection 唯一 switch |
| `ProviderAdapter` | crate-private trait | protocol + complete against `OperationClient` | 不暴露给 Core/public config |
| `api_endpoint` | private function | `ApiProfile → zeta_api::ApiEndpoint` | 按 profile，不按 provider name 猜 |
| provider `*Adapter::new` | crate-private | normalized base URL + fixed headers + endpoint | one immutable runtime snapshot |
| `Provider::resolve_model` | private method | listed lookup 或 allow-unlisted synthetic model | 不做远端 catalog request |
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
      └─ RegisteredModelInvoker { provider, model }

RegisteredModelInvoker::invoke(request)
├─ clone canonical ModelRequest
├─ apply normalized max_output_tokens when the request has no explicit limit
└─ Provider::complete
   ├─ resolve_model
   └─ ProviderAdapter::complete
      └─ zeta_api::ApiEndpoint::complete_with_client
         └─ OperationClient
```

`Provider::complete` 再次 resolve model，因此 direct Provider callers 与 bound invoker 使用相同
catalog policy。

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
| wire/operation codec failure | `Api(ApiError)` |
| explicitly unavailable host model | `Unavailable` |

`AllowUnlisted` 只创建 `Model::new(id, id)` 作为 runtime selection，不证明 entitlement、capability 或
remote availability。`ListedOnly` 在任何 network call 前拒绝。

## 方向偏差检查

- Core 按 provider ID branch：provider selection 泄漏出 runtime crate；
- runtime 根据 URL 猜 profile：declarative config 被绕过；
- provider adapter 自己序列化 wire JSON：codec ownership 从 `zeta-api` 漂移；
- adapter 直接构造 ureq/reqwest client：network substrate/retry ownership 漂移；
- `RegisteredModelInvoker` 读取 mutable config：safe-point snapshot 被破坏；
- `EchoModel` 出现在 production fallback：配置失败被静默掩盖；
- API/network layer 读取 secret store：credential lifecycle 下沉到错误层。

## 测试、限制与演进

```text
cargo test -p zeta-model-provider
bazel test //zeta-rs/model-provider:model-provider-unit-tests
```

测试使用注入的 `OperationClient` 捕获请求，覆盖 Responses/Chat/Anthropic 配置、结构化工具、
自定义端点、默认值、供应商不匹配、目录策略、固定标头、取消传播和默认 HTTP 传输。

当前 completion `ModelInvoker` 已有 concrete provider adapters；embedding/rerank 已有 canonical invoker、
request/response validation 和 CodeIndex service consumer，但尚无 concrete provider codec/runtime resolver。
当前 invocation 是同步 unary；
credential materialization、subscription backend、streaming 与动态
catalog 的长期设计仍在系统文档中演进。新增能力应保持 invoker immutable、profile explicit、
provider adapter private，以及 config/codec/operation/network 四层分离。

`RegisteredModelInvoker::invoke_with_cancellation` 把 caller token 逐层传给 private
`ProviderAdapter::complete` 和 `OperationClient::execute_with_cancellation`。取消是独立的
`ModelProviderError::Cancelled`，App Server 不会把它误报为 model failure。同步 HTTP attempt
可能已进入底层 socket；operation 立即停止等待且不 retry，attempt 本身由 transport timeout
有界结束，迟到 response 不会被接受。
