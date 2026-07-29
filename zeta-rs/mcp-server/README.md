# zeta-mcp-server

> Current crate implementation contract. Cross-crate product architecture and staged evolution:
> [`docs/mcp-server.md`](../../docs/mcp-server.md). MCP client runtime in the opposite direction:
> [`docs/mcp.md`](../../docs/mcp.md).

`zeta-mcp-server` exposes Zeta Agent execution to MCP clients over stdio or authenticated
Streamable HTTP. It is an adapter over `zeta-app-server-client`; the embedded App Server remains
the only composition root and the canonical owner of Session, Thread, Turn, model, Tool, policy,
workspace and durable state.

## 1. Current surface

Start a local stdio server:

```text
zeta-mcp-server
zeta mcp-server
zeta mcp-server --listen stdio://
```

Start an HTTP endpoint:

```text
ZETA_MCP_BEARER_TOKEN=<at-least-32-visible-ASCII-characters> \
  zeta mcp-server --listen http://127.0.0.1:8787/mcp
```

Both entry points read `ZETA_STATE_ROOT`, defaulting to `.zeta`, and
`ZETA_WORKSPACE_ROOT`, defaulting to the process working directory. HTTP additionally requires
`ZETA_MCP_BEARER_TOKEN`; `ZETA_MCP_ALLOWED_ORIGIN` optionally permits one exact browser origin.
The built-in listener is plain HTTP and should be placed behind an authenticated TLS reverse proxy
for remote deployment.

The server implements MCP `2025-11-25`:

| Method | Current behavior |
| --- | --- |
| `initialize` | advertises Tool capability and negotiates form elicitation support |
| `ping` | returns an empty result |
| `tools/list` | returns `zeta` and `zeta-reply` |
| `tools/call` | starts or continues one App Server Thread |
| `notifications/cancelled` | requests interruption of the correlated Turn |
| server `elicitation/create` | forwards approval and user-input requests to capable clients |
| `notifications/progress` | projects bounded, redacted Turn lifecycle updates |

`zeta` requires a caller-generated `invocationId` and a prompt. `zeta-reply` requires a new
`invocationId`, a `threadId` authorized for the same principal and a prompt. Both return stable Zeta
identities, terminal or blocked status and bounded content in `structuredContent`.

## 2. Crate boundary

The crate owns:

- MCP framing, initialize gate, Tool schemas and wire validation;
- stdio lifecycle and Streamable HTTP endpoint/session lifecycle;
- bearer authentication, exact Origin validation and per-process connection limiting;
- request cancellation and progress-token correlation;
- App Server interaction to MCP form-elicitation projection;
- principal-scoped durable invocation receipts and caller-to-Thread bindings;
- App Server result projection and output truncation.

It does not own:

- Session/Thread/Turn state transitions or storage;
- model, Tool, policy, sandbox, credential or workspace authority;
- local parent/child delegation;
- App Server protocol definitions;
- OAuth, tenant provisioning, TLS termination or a remote App Server backend.

Code that directly opens product stores, constructs `TurnExecutor`, invokes providers or writes
Thread events from this crate is architectural drift. Those operations remain behind the App
Server client.

## 3. Modules and key symbols

| Module/symbol | Responsibility | Must not absorb |
| --- | --- | --- |
| `lib.rs::{run_stdio,run_http}` | validate options, open one embedded App Server host and receipt store | protocol dispatch or Agent state |
| `options.rs::{McpServerOptions,HttpServerOptions}` | host-owned roots, runtime limits and HTTP security configuration | caller permission overrides |
| `server.rs::McpServer` | initialize gate, JSON-RPC dispatch and active-call cancellation | Session/Thread business logic |
| `server/events.rs::McpAgentEvents` | progress and elicitation wire projection | policy decisions |
| `http.rs` / `http/wire.rs` | authenticated endpoint, MCP sessions and SSE framing | durable Agent authority |
| `protocol.rs` | wire DTOs, Tool schemas and input limits | App Server DTO passthrough |
| `agent.rs::AgentService` | narrow testable Agent execution boundary | model/Tool implementation |
| `agent.rs::AppServerAgentService` | map start/reply to typed App Server calls and exact Turn updates | direct Core/store access |
| `agent/progress.rs` | redact and bound Thread updates for MCP progress | full transcript projection |
| `interaction.rs` | typed approval/user-input and MCP form-elicitation mapping | automatic approval |
| `receipt.rs::ReceiptStore` | principal-scoped replay, single-flight and Thread authorization | product transcript storage |
| `agent/outcome.rs` | terminal/waiting Turn projection | Thread mutation or MCP framing |

The current call path is:

```text
run_stdio or run_http
→ open_in_process_app_server
→ create a principal-scoped AppServerAgentService
→ McpServer::handle_message
→ tools/call
→ session/create or authorized thread/read
→ session/thread/create when starting
→ thread/subscribe + turn/start
→ bounded thread/read polling + notification draining
→ progress and optional elicitation/create
→ turn/interaction/resolve when accepted
→ bounded MCP CallToolResult
```

Each HTTP MCP session gets a separate App Server connection but shares the same embedded App Server
host and receipt authority.

## 4. Validation and limits

- `invocationId`: 1–128 ASCII letters, digits, `.`, `_` or `-`;
- prompt: non-blank and at most 256 KiB;
- default Turn timeout: 60 seconds;
- maximum caller-requested Turn timeout: 10 minutes;
- polling interval: 10 milliseconds;
- MCP Tool result content: at most 256 KiB including the truncation marker;
- progress: at most 256 lifecycle notifications per call with consecutive duplicates removed;
- HTTP request body: at most 1 MiB; headers: at most 32 KiB;
- default maximum HTTP connections: 64;
- early cancellation identities: at most 1024 per MCP session.

The server does not accept a workspace path, raw config map, secret, developer instruction or
sandbox override through Tool arguments. Workspace and execution authority are fixed by the host.

## 5. Identity, recovery and continuation

`invocationId` is separate from MCP JSON-RPC request ID and all Zeta product identities. The
adapter derives principal-namespaced stable App Server command IDs for Session, Thread, Turn,
interaction resolution and cancellation.

Receipts are atomically persisted at
`<ZETA_STATE_ROOT>/mcp-server/receipts-v1.json` and scoped by principal:

- stdio uses the local-user principal;
- HTTP derives a non-reversible principal identifier from the bearer token;
- a finished invocation with identical arguments replays its saved outcome;
- the same identity with different arguments returns a conflict;
- a concurrent duplicate returns in-progress;
- an invocation left running by process failure re-enters the same deterministic App Server
  commands after restart rather than allocating new product identities;
- `zeta-reply` accepts only Threads durably bound to the same principal.

The receipt file is an adapter recovery index, not the canonical Agent state. Session/Thread/Turn
state remains in App Server storage. A state root currently supports one MCP server process at a
time; cross-process file locking and distributed receipt storage are not implemented.

Waiting-for-interaction outcomes remain resumable rather than being sealed as finished. A retry
with the same invocation can continue the exact outstanding interaction after restart.

## 6. Progress, interaction and failure

When the caller provides `_meta.progressToken`, the server emits monotonically increasing
`notifications/progress` with the exact token. Messages describe bounded lifecycle state only; they
do not expose reasoning, prompt text, Tool arguments/results, credentials or environment data.

Approval and user-input requests are mapped to MCP `elicitation/create` only when the initialized
client declares form elicitation. An accepted response is converted back to the exact typed App
Server request identity and sent through `turn/interaction/resolve`. Decline, cancellation or an
unsupported client returns a blocked Tool result and never auto-approves. User-input questions
that appear to request credentials or other sensitive values are not sent through form elicitation.
Dynamic Tool-defined interaction kinds are not yet projected.

Stdio keeps reading while a long call runs, so `notifications/cancelled` can interrupt the exact
Turn. Stdio EOF cancels active work because the process owns that connection. An HTTP/SSE write
failure does not prove the durable Turn was cancelled; explicit MCP cancellation remains the
authority. Client-cancelled requests suppress their JSON-RPC response.

Cancellation has a two-second grace period. If a server deadline fires and the Turn does not reach
a canonical terminal state, the result is `outcomeUnknown`. Invalid JSON-RPC or protocol requests
use JSON-RPC errors; Tool argument, App Server and Agent outcome failures use
`CallToolResult.isError`.

## 7. Streamable HTTP security and lifecycle

The HTTP endpoint:

- accepts `POST` for MCP messages and returns JSON or SSE;
- returns `202 Accepted` for notifications and client responses;
- uses secure random `MCP-Session-Id` values after successful initialize;
- requires `MCP-Protocol-Version: 2025-11-25` on subsequent requests;
- supports `DELETE` to terminate a session;
- returns `405` for independent `GET` streams;
- validates an exact configured `Origin` when the header is present;
- compares bearer credentials without an early-exit string comparison.

HTTP session state is intentionally process-local. After server restart, the client initializes a
new MCP session and retries the same durable `invocationId`. Independent GET SSE streams,
`Last-Event-ID` redelivery, OAuth, multi-tenant workspace binding and built-in TLS are not
implemented.

## 8. Tests and extension points

Sibling unit/integration modules cover protocol validation, progress tokens, elicitation identity,
real App Server start/reply/progress, durable receipt reopen, HTTP auth/origin/session/protocol/SSE,
cancellation and truncation. `tests/stdio.rs` launches the real binary and verifies live progress,
restart replay and `zeta-reply` after restart.

Run:

```text
cargo test -p zeta-mcp-server
cargo clippy -p zeta-mcp-server --all-targets -- -D warnings
```

Current extension points and limitations:

- synchronous App Server requests plus bounded Thread polling/draining are still used; the proposed
  owned async `AppServerSession` would provide a general independent event driver;
- no resource, prompt, root, sampling or MCP task capability;
- no artifact reference for oversized output;
- no dynamic Tool interaction projection;
- no HTTP event replay, OAuth/tenant control plane, built-in TLS or remote App Server backend;
- no native remote-Agent-to-`DelegationId` bridge.

Changes to Tool schemas, limits, status mapping, receipt identities, HTTP security or continuation
authorization require synchronized updates to fixtures, tests, this README and
[`docs/mcp-server.md`](../../docs/mcp-server.md).
