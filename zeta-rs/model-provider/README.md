# Model providers

This is the source of truth for the providers compiled into Zeta. Each provider is one
authentication, protocol, endpoint, and configuration boundary. Models are mounted under that
provider and are uniquely selected by `(ProviderId, ModelId)`:

```json
{
  "provider": "openai",
  "model": "gpt-5.6"
}
```

Provider modules own their default endpoint, authentication strategy, fixed request headers, and
mounted model metadata. Providers backed by dynamic or user-managed catalogs may accept model IDs
that are not statically listed; the registry still resolves them under the selected provider.
Re-check the linked documentation before changing a profile, because provider API contracts evolve
independently.

Wire request and response conversion is implemented by provider adapters in the sibling
`zeta-api` crate. Each registration selects its own concrete `zeta_api::Api` variant. Compatible
providers may share a lower-level codec, but retain separate adapter entry points for their
provider-specific behavior.

| Provider | ID | Default base URL | API profile | Reference |
| --- | --- | --- | --- | --- |
| OpenAI (GPT) | `openai` | `https://api.openai.com/v1` | Responses API, Bearer | [Responses API](https://developers.openai.com/api/reference/resources/responses/methods/create) |
| OpenAI-compatible | `openai-compatible` | Custom URL required | Chat Completions, Bearer | Provider-specific compatibility documentation |
| Anthropic (Claude) | `anthropic` | `https://api.anthropic.com` | Messages API, `x-api-key` and version header | [Messages API](https://docs.anthropic.com/en/api/messages) |
| Google (Gemini) | `google` | `https://generativelanguage.googleapis.com/v1beta/openai` | Chat Completions, Bearer, Google client header | [OpenAI compatibility](https://ai.google.dev/gemini-api/docs/openai) |
| xAI (Grok) | `xai` | `https://api.x.ai/v1` | Chat Completions, Bearer | [Chat Completions](https://docs.x.ai/developers/model-capabilities/legacy/chat-completions) |
| Qwen | `qwen` | `https://dashscope.aliyuncs.com/compatible-mode/v1` | Chat Completions, Bearer | [OpenAI-compatible Chat](https://help.aliyun.com/en/model-studio/compatibility-of-openai-with-dashscope) |
| Kimi | `kimi` | `https://api.moonshot.ai/v1` | Chat Completions, Bearer | [API overview](https://platform.kimi.ai/docs/api/overview) |
| DeepSeek | `deepseek` | `https://api.deepseek.com` | Chat Completions, Bearer | [API quickstart](https://api-docs.deepseek.com/) |
| Ollama | `ollama` | `http://localhost:11434/v1` | Local Chat Completions, no authentication header | [OpenAI compatibility](https://docs.ollama.com/api/openai-compatibility) |
| Hugging Face | `huggingface` | `https://router.huggingface.co/v1` | Chat Completions, Bearer | [Inference Providers](https://huggingface.co/docs/inference-providers/index) |
| Z.AI (GLM) | `zai` | `https://api.z.ai/api/paas/v4` | Chat Completions, Bearer and language header | [Chat Completion API](https://docs.z.ai/api-reference/llm/chat-completion) |
| MiniMax | `minimax` | `https://api.minimax.io/v1` | Chat Completions, Bearer | [API documentation](https://platform.minimax.io/docs) |
| Xiaomi MiMo | `mimo` | `https://api.xiaomimimo.com/v1` | Chat Completions, Bearer | [Chat Completions compatibility](https://mimo.mi.com/docs/en-US/api/chat/openai-api) |
