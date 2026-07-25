# Zeta

Zeta is a Rust-first agent system with CLI, TUI, and app-server product entries.

The Rust workspace lives in [`zeta-rs`](zeta-rs); Electron is a separate client under
[`desktop`](desktop).

Team responsibilities and integration contracts are documented separately:

- [Desktop architecture](docs/zeta-desktop-architecture.md)
- [CLI architecture](docs/zeta-cli-architecture.md)
- [zeta-rs architecture and public surfaces](docs/zeta-rs-architecture.md)
- [Accepted App Server API v1](docs/zeta-app-server-api-v1.md)
- [API contract requirements](docs/zeta-api-interface-requirements.md)
- [API contract template](docs/zeta-api-interface-template.md)

The original
[`zeta-code-architecture-codex-style-v2.md`](docs/zeta-code-architecture-codex-style-v2.md)
is retained as the historical unified design.

## Run

```bash
cargo run --manifest-path zeta-rs/Cargo.toml -p zeta-cli -- ask "explain this repository"
cargo run --manifest-path zeta-rs/Cargo.toml -p zeta-cli -- exec "summarize the current changes"
```

## Model-provider setup

The local App Server resolves models through a provider registry rather than hard-coding a
provider into the CLI. The supported providers, endpoint behavior, request authentication, and
reference documentation live in the [model-provider README](zeta-rs/model-provider/README.md).
Zeta's normalized model protocol and provider-specific wire adapters live in
[`zeta-api`](zeta-rs/zeta-api); `zeta-model-provider` owns registration, model catalogs,
authentication, endpoints, and fixed provider headers.
Store each API key in macOS Keychain under the `zeta` service and the configured account. Keys
never belong in `.zeta/config.json` or Thread rollout logs.

Registered providers use their documented default endpoint when `baseUrl` is omitted or empty;
an explicitly configured `baseUrl` overrides that default. `openai-compatible` has no safe default
and therefore requires an explicit endpoint. Ollama is the exception to credential handling: its
default local endpoint sends no authentication header.

### OpenAI-compatible providers

The `openai` provider sends a non-streaming request to `/responses`. Other providers in this
section currently use their documented OpenAI-compatible `/chat/completions` endpoint.
Select the service and model together through `preferredModel`. The provider identifies the
authentication, protocol, and endpoint boundary; the model is resolved from that provider's
catalog (for example, `openai`, `google`, `xai`, `qwen`, `kimi`, `deepseek`, `ollama`,
`huggingface`, `zai`, `minimax`, or `mimo`):

```json
{
  "preferredModel": {
    "provider": "openai",
    "model": "gpt-5.6"
  },
  "modelProvider": {
    "baseUrl": "https://api.openai.com/v1",
    "credentialAccount": "openai-api-key"
  }
}
```

### Anthropic

Anthropic uses `POST /v1/messages`, API-key authentication, and its version header. Configure it
with the provider's root API URL (not a Chat Completions URL):

```json
{
  "preferredModel": {
    "provider": "anthropic",
    "model": "claude-sonnet-4-20250514"
  },
  "modelProvider": {
    "baseUrl": "https://api.anthropic.com",
    "credentialAccount": "anthropic-api-key",
    "maxOutputTokens": 4096
  }
}
```

For example, use Keychain Access to create a generic password with service `zeta` and account
`openai-api-key` or `anthropic-api-key` as selected above.

## Bazel

The Rust workspace is also built through Bazel using the Cargo.lock-derived
dependency graph, matching Codex's Bzlmod and `rules_rs` integration.

```bash
bazel build //...
bazel test //...
bazel run //:zeta -- ask "explain this repository"
```

Use `user.bazelrc` for machine-specific Bazel settings; it is intentionally not
tracked.
