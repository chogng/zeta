# Zeta

Zeta is a Rust-first agent system with CLI, TUI, and app-server product entries.

The Rust workspace lives in [`zeta-rs`](zeta-rs); Electron is a separate client under
[`desktop`](desktop).

Team responsibilities and integration contracts are documented separately:

- [Desktop architecture](docs/zeta-desktop-architecture.md)
- [CLI architecture](docs/zeta-cli-architecture.md)
- [zeta-rs architecture and public surfaces](docs/zeta-rs-architecture.md)
- [Secret storage architecture](docs/secrets.md)
- [Provider runtime and authentication architecture](docs/model-provider.md)
- [App Server API](docs/zeta-app-server-api.md)
- [API contract requirements](docs/zeta-api-interface-requirements.md)
- [API contract template](docs/zeta-api-interface-template.md)

The long-term domain and dependency boundaries are defined in
[`zeta-code-architecture-codex-style-v2.md`](docs/zeta-code-architecture-codex-style-v2.md).

## License

Zeta's original code and materials are proprietary and all rights reserved.
See [`LICENSE`](LICENSE). Third-party components remain governed by their own
licenses and notices, including the Desktop notices in
[`desktop/THIRD_PARTY_NOTICES.md`](desktop/THIRD_PARTY_NOTICES.md).

## Run

With [`just`](https://just.systems/) installed, launch the interactive terminal from the current
source tree:

```bash
just zeta
```

The equivalent Cargo command is:

```bash
cargo run --manifest-path zeta-rs/Cargo.toml -p zeta-cli
```

Run a one-shot prompt:

```bash
just zeta ask "explain this repository"
just zeta exec "summarize the current changes"

# Without just:
cargo run --manifest-path zeta-rs/Cargo.toml -p zeta-cli -- ask "explain this repository"
cargo run --manifest-path zeta-rs/Cargo.toml -p zeta-cli -- exec "summarize the current changes"
```

## Package

Build a canonical package directory containing Zeta, its built-in Skills, and
the pinned ripgrep runtime:

```bash
python3 scripts/build_zeta_package.py \
  --package-dir /absolute/path/to/zeta-package
```

The builder compiles a release `zeta` executable when `--zeta-bin` is omitted,
downloads the target-specific ripgrep archive, verifies its locked size and
SHA-256 digest, and creates `bin/`, `zeta-path/`, `zeta-resources/`, and
`zeta-package.json`. Repository-owned Skills are staged from
`zeta-rs/skills/assets/` to `zeta-resources/skills/`. Linux packages additionally
build and include the locked Bubblewrap runtime; Windows packages build and
include both first-party AppContainer helpers. Cross-platform release jobs pass
`--target`; jobs that already built or signed binaries pass the corresponding
Zeta, rg, Bubblewrap, or Windows helper override flags. The exact layout and
failure contract are documented in the
[package builder README](scripts/zeta_package/README.md).

## Model-provider setup

The local App Server resolves models through a provider registry rather than hard-coding a
provider into the CLI. The supported providers, endpoint behavior, request authentication, and
reference documentation live in the [model-provider architecture](docs/model-provider.md).
Zeta's normalized model contract lives in `zeta-protocol`; endpoint/request/event codecs live in
[`zeta-api`](zeta-rs/zeta-api). `zeta-model-provider` owns runtime registration, model selection,
credential binding, Provider auth/login, resolved targets, and fixed runtime headers; opaque secret
persistence belongs to [`zeta-secrets`](docs/secrets.md). Dynamic catalog policy belongs to
the proposed `zeta-models-manager`; shared proxy/TLS/HTTP/WebSocket transport belongs to
[`zeta-http-client`](zeta-rs/http-client/README.md), while operation retry/framing belongs to
`zeta-client`.
Production hosts store each API key through a configured `zeta-secrets` OS-keyring backend. Keys
never belong in `.zeta/config.json`, App Server DTOs, or Thread rollout logs.

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
