# zeta-session-store

`zeta-session-store` defines the storage-neutral durability boundary for Session structure.
It stores only durable `SessionEvent` facts and typed `SessionCommandReceipt` values. Runtime
mailboxes, update queues, JSON-RPC DTOs, filesystem layout, and Thread transcripts stay outside
this crate.

Each Session owns an independent sequence. A batch is atomic: implementations either durably
commit every event or expose none of them, and must reject stale expected sequences.
