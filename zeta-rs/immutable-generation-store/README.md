# `zeta-immutable-generation-store`

1. The store publishes immutable base generations and immutable change-layer snapshots into generation-specific directories; a versioned manifest is written last, and a store-wide file lock serializes readers with publishers across processes.
2. `PublishedSnapshot` reads or opens files only from the selected base and layer. Mapped base files retain a shared generation lease, so cleanup cannot remove their generation until every reader drops it; published files are never truncated or overwritten in place.
3. This crate owns persistence publication, leases and the sole audited file-mapping operation for application-owned cache files that outside processes must not rewrite. It is not a sandbox and does not own path authorization, index formats, file watching or corrupt-data recovery policy.
