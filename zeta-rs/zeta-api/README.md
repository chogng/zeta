# zeta-api

`zeta-api` owns Zeta's normalized model API and the adapters that translate it to concrete
provider wire protocols. Its source tree deliberately separates those two responsibilities:

```text
src/
├── request.rs
├── response.rs
├── error.rs
├── transport.rs
├── openai.rs
├── openai/
│   └── responses.rs
├── openai_compatible.rs
├── openai_compatible/
│   └── completions.rs
├── anthropic.rs
├── anthropic/
│   └── messages.rs
├── qwen.rs
├── qwen/
│   └── completions.rs
├── google.rs
├── google/
│   └── completions.rs
└── ...
```

Each provider has a `provider.rs` module entry and a matching `provider/` implementation
directory. Providers whose wire format is currently OpenAI-compatible reuse the shared
Completions codec through that entry point, so future request, response, streaming, or error
differences can be added without changing the registry or duplicating the codec.

The implemented wire protocols are:

- OpenAI Responses;
- OpenAI-compatible Chat Completions;
- Anthropic Messages.

The normalized API covers messages, function tools, tool results, reasoning settings, usage, stop
reasons, and provider output items. Its HTTP transport keeps credential-bearing header values out
of debug output.

Provider identity, model catalogs, endpoint defaults, authentication configuration, and fixed
provider headers belong to `zeta-model-provider`. Agent and tool execution loops belong to
`zeta-core`.

The current transport is non-streaming. The normalized response retains text, reasoning, tool
calls, usage, and stop reasons, but SSE/WebSocket streaming must be implemented explicitly before
the API is described as streaming-capable.
