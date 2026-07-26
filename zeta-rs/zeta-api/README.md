# zeta-api

The canonical architecture and evolution plan is
[`docs/zeta-api.md`](../../docs/zeta-api.md).

`zeta-api` is the model API protocol layer. Canonical model values belong to `zeta-protocol`; this
crate translates those values to and from concrete endpoint, request/response, and streaming event
protocols. Shared HTTP/WebSocket execution and network policy belong to `zeta-http-client`;
operation retry, SSE/NDJSON framing, and operation telemetry belong to `zeta-client`.

```text
src/
├── endpoint/
├── requests/
├── sse/
├── ndjson/
└── error/
```

Canonical request and response values are defined in `zeta-protocol` and explicitly re-exported
from this crate; they are not duplicated in local `request.rs` or `response.rs` modules.

The tree above is the migration target, not the current implementation. Provider registry and
selection live only in `zeta-model-provider`. This crate is organized by API endpoint/profile;
compatible profiles may share mechanical codecs without claiming identical cache, error, usage,
or streaming behavior.

A provider can expose more than one official profile (for example, xAI supports Chat Completions
and Responses; Ollama supports compatible SSE and native NDJSON). Profile selection belongs to
validated model-provider runtime configuration, not URL or provider-name guessing. Heartbeats,
cache controls, errors, usage, and catalog behavior remain profile-specific even when the
invocation body is OpenAI-compatible.

The implemented wire protocols are:

- OpenAI Responses;
- OpenAI-compatible Chat Completions;
- Anthropic Messages.

The normalized API covers messages, function tools, tool results, reasoning settings, usage, stop
reasons, and provider output items. Credential-bearing headers and raw payloads must remain out of
protocol diagnostics.

Provider registry and selection, endpoint defaults, authentication configuration, credential or
deployment headers, and retry policy selection belong to `zeta-model-provider`. Catalog refresh,
cache, merge, and filtering belong to the proposed `zeta-models-manager`; model-list wire codecs
may live here. Protocol-required headers such as API versions and streaming media types belong to
the corresponding `zeta-api` endpoint. Agent and tool execution loops belong to `zeta-core`.

The current implementation remains synchronous and non-streaming. The normalized response retains
text, reasoning, tool calls, usage, and stop reasons, but `zeta-client` framing plus API event
decoders must be implemented before the stack is described as streaming-capable.
