# zeta-collaboration

`zeta-collaboration` owns the backend-neutral authority for ordered Gama
structured-document rooms. It does not own HTTP, App Server JSON-RPC,
Workbench state, sessions, users, or document-schema semantics.

`InMemoryDocumentCollaborationRooms` is composed by the App Server for clients
in one process. `SqliteDocumentCollaborationRooms` is composed by the remote
host and persists room snapshots and ordered submissions across restarts. Both
implement the same `open`, `submit`, and `replay` operations.

## Contract

- `open` creates an unpredictable `gama-` room ID or joins an existing room
  only when its schema compatibility ID matches.
- `submit` validates bounded JSON-object envelopes, assigns a monotonic
  JavaScript-safe version, and returns `Accepted`, bounded rebase history, or
  a canonical resync snapshot.
- A repeated `(roomId, clientId, sequence)` is accepted only when its base
  version, transaction, and resulting document are identical. The original
  accepted update is returned; a changed reuse is rejected.
- `replay` returns at most the newest 512 ordered updates. A client older than
  that window receives the current snapshot instead.

SQLite keeps submitted-operation rows after they leave the 512-update replay
window so an old sequence can never be reused. Retention/compaction therefore
needs an explicit future protocol for expiring client identities; deleting
those rows without such a contract would break idempotency.

The crate validates JSON shape and byte limits, not the full Gama schema.
`RemoteDocumentCollaborationService` decodes every server value through the
active Gama schema before it can reach a `DocumentModel`. A Rust schema
authority belongs here only once the schema is independently represented in
the backend rather than copied from the browser.

## Tests

`cargo test -p zeta-collaboration` covers room creation, ordered replay,
exact retries, and reopening a SQLite database. App Server adapter coverage is
in `zeta-rs/app-server/src/server_tests.rs`.
