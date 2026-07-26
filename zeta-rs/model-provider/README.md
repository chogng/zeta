# Model providers

The canonical architecture and evolution plan is
[`docs/model-provider.md`](../../docs/model-provider.md).

This crate turns validated declarative configuration into runnable provider instances. Models are
uniquely selected by `(ProviderId, ModelId)`:

```json
{
  "provider": "openai",
  "model": "gpt-5.6"
}
```

The sibling `zeta-model-provider-config` crate owns serializable configuration, schemas, static
validation, normalization rules, built-in definitions, and registry merging. This runtime crate
owns endpoint resolution results, provider-specific API profile selection,
credential/deployment runtime headers, retry policy selection, and runtime errors.
Protocol-required headers and Provider wire codecs belong to `zeta-api`; shared HTTP/WebSocket
execution and network policy belong to `zeta-http-client`, while operation retry and SSE/NDJSON
framing belong to `zeta-client`.

Wire request, response, and event conversion is implemented by endpoint/profile codecs in the
sibling `zeta-api` crate. Provider selection happens only in this crate. Compatible providers may
share an API codec, but the runtime must select any verified compatibility profile explicitly.

Provider-specific runtime adapters are kept in `src/providers/`, one module per external service.
Zeta uses the provider-neutral `ModelProvider` interface in `src/provider.rs`; callers submit a
model selection and `ModelProviderConfig`, without constructing API clients or branching on an
adapter identity. `ModelProviderRuntime` is the concrete process-local implementation and owns the
configuration registry plus a client handle. There is intentionally no second Provider registry
in `zeta-api`.

Boundary rules:

- `zeta-model-provider-config` must not depend on API clients, transports, credentials, or Core.
- `zeta-model-provider` may depend on configuration declarations, but the configuration crate must
  never depend back on the runtime crate.
- endpoint defaults belong to provider definitions; resolved endpoints belong to runtime
  `Provider` instances.
- adapter identifiers are serializable declarations; concrete `zeta-api` endpoint/profile bindings
  and fixed runtime headers are process-local values.
- configuration failures are returned as `ProviderConfigError`; adapter and transport failures are
  returned as `ModelProviderError`.
- configuration may serialize credential references or auth policy, but secret lookup, token
  refresh, scope validation, and header/signature materialization remain runtime behavior.

Direct-provider credential lifecycle, including API-key materialization, cloud-identity adapters,
scope validation, refresh, and request signing, belongs to this runtime crate. Opaque secret
persistence is delegated to the sibling `zeta-secrets` crate; API and network client layers never
read the secret store. The exact long-term ownership is documented in
[`docs/model-provider.md`](../../docs/model-provider.md#6-provider-credential-与-subscription-backend).

ChatGPT/Codex subscription login and credential refresh do **not** belong to this crate. Zeta uses
an injected subscription backend implemented by `zeta-codex-app-server`, which controls the local
upstream Codex App Server; its public architecture is in
[`docs/codex-app-server.md`](../../docs/codex-app-server.md). This crate never reads Codex token
storage or invokes the private ChatGPT/Codex backend directly.

The local App Server resolves a fresh immutable model runtime from the latest authoritative Config
at the start of every model invocation. A configuration update therefore affects the next
invocation without mutating one already in flight.

| Provider | ID | Default base URL | Runtime API profile | Reference |
| --- | --- | --- | --- | --- |
| OpenAI (GPT) | `openai` | `https://api.openai.com/v1` | Responses API | [Responses API](https://developers.openai.com/api/reference/resources/responses/methods/create) |
| OpenAI-compatible | `openai-compatible` | Custom URL required | Chat Completions | Provider-specific compatibility documentation |
| Anthropic (Claude) | `anthropic` | `https://api.anthropic.com` | Messages API and version header | [Messages API](https://docs.anthropic.com/en/api/messages) |
| Google (Gemini) | `google` | `https://generativelanguage.googleapis.com/v1beta/openai` | Current Chat Completions profile; native Gemini is a separate future profile | [OpenAI compatibility](https://ai.google.dev/gemini-api/docs/openai) |
| xAI (Grok) | `xai` | `https://api.x.ai/v1` | Chat Completions | [Chat Completions](https://docs.x.ai/developers/model-capabilities/legacy/chat-completions) |
| Qwen | `qwen` | `https://dashscope.aliyuncs.com/compatible-mode/v1` | Chat Completions | [OpenAI-compatible Chat](https://help.aliyun.com/en/model-studio/compatibility-of-openai-with-dashscope) |
| Kimi | `kimi` | `https://api.moonshot.ai/v1` | Configured Chat Completions contract; advanced wire behavior remains unverified | [Kimi API Platform](https://platform.moonshot.ai/docs/) |
| DeepSeek | `deepseek` | `https://api.deepseek.com` | Chat Completions | [API quickstart](https://api-docs.deepseek.com/) |
| Ollama | `ollama` | `http://localhost:11434/v1` | Local Chat Completions | [OpenAI compatibility](https://docs.ollama.com/api/openai-compatibility) |
| Hugging Face | `huggingface` | `https://router.huggingface.co/v1` | Chat Completions | [Inference Providers](https://huggingface.co/docs/inference-providers/index) |
| Z.AI (GLM) | `zai` | `https://api.z.ai/api/paas/v4` | Chat Completions and language header | [Chat Completion API](https://docs.z.ai/api-reference/llm/chat-completion) |
| MiniMax | `minimax` | `https://api.minimax.io/v1` | Current Chat Completions profile; official Anthropic-compatible profile is a separate option | [API documentation](https://platform.minimax.io/docs) |
| Xiaomi MiMo | `mimo` | `https://api.xiaomimimo.com/v1` | Configured Chat Completions contract; official wire reference still required | No verified public reference |
