# `zeta-immutable-generation-store`

1. The store publishes immutable base generations and change-layer snapshots under a cross-process write lock. Publication uses compare-and-set against the expected snapshot, records a content digest for exact retries, and distinguishes pre-commit failure from a committed manifest whose directory durability is unknown.
2. `PublishedSnapshot` retains shared generation leases and exposes full reads or positioned `read_exact_at` access. This crate contains no mmap, shared file cursor, or unsafe code; a format-specific consumer may map an opened immutable file while retaining its lease.
3. Cleanup removes stale manifests and syncs that directory before deleting unreferenced layer/base data, treats missing paths as success, and returns cleanup facts or errors to its caller. Process-abort tests cover process-crash recovery; they do not claim to simulate machine power loss.
