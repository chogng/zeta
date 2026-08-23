# `zeta-codex-app-server`

This crate owns the local process and JSON-RPC adapter for upstream Codex App Server capabilities.
Its current implementation covers managed ChatGPT account/login and complete Codex thread/Turn
delegation. OAuth, callbacks, device-code polling, credential persistence, token refresh, and the
remote agent loop remain entirely inside upstream Codex.

## Current execution path

`CodexAppServerRuntime` lazily starts `codex app-server --listen stdio://` through
`CodexAppServerProcess::start`. The shared runtime performs the `initialize`/`initialized` handshake,
routes bounded JSONL responses to exact request IDs, and sends account and Turn events to separate
listeners so event handling never deadlocks stdout.

`CodexAppServerLoginDriver` maps Zeta-owned `LoginId` values to opaque upstream IDs. Early completion
notifications are retained until that mapping is installed. Successful completion reads only the
upstream redacted account projection; upstream error text and all credential fields are discarded.

`zeta-model-provider-config::STATIC_MODEL_CATALOG` owns the static subscription models exposed by
Zeta. `CodexModelCatalog` remains an explicit adapter for callers that need the upstream account
catalog, but the default product model list and Session selection do not use it as a health check.
`CodexTurnDriver` exposes typed thread/Turn streaming, command and file-change approvals, structured
user input, interruption, and exact once-only server-request resolution.
`CodexTurnExecutionBackend` implements Core's `TurnExecutionBackend`: Core remains authoritative for
durable Thread state, interactions, cancellation, and terminal outcomes while Codex owns its remote
agent loop. A remote thread binding, including its opaque Workspace authority scope, is persisted only
after a successful Turn. The default App Server routes a Turn here only when its persisted model's
unique static catalog row declares subscription access; login state never changes execution
implicitly. The model provider remains `openai`; `openai-chatgpt` is only the managed-login account
adapter identity.

## Failure semantics

- spawn, pipe, framing, timeout, and process-exit failures become stable unavailable errors;
- missing methods and incompatible response shapes fail explicitly and never fall back to private
  ChatGPT backend calls;
- failed inference, approval, user-input, or login requests are not replayed after an unknown process
  outcome;
- connection generations prevent late server requests from being answered on a restarted process;
- unsupported secret user input, local image input, or approval semantics fail closed;
- stderr is not captured into RPC errors or telemetry.

The product App Server selects this backend through the canonical static model catalog and Session
model selection. Account, entitlement, rate-limit, transport, and model support failures are decided
by the real thread/Turn invocation and become durable Turn errors in the conversation; they are not
preflight availability flags. Permission approval, diff projection, images, secret user input,
rate-limit observation, and richer completed-item projection remain explicit follow-up slices.
