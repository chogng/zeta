# zeta-plugin-marketplace

`zeta-plugin-marketplace` owns verified remote distribution for product-managed Plugin
Marketplaces. The cross-crate Plugin/Connector/MCP product model is canonical in
[`docs/plugins.md`](../../docs/plugins.md); this README documents the exact implementation
contract of this crate.

## Ownership

`RemotePluginMarketplace` refreshes a TUF repository with a host-pinned root, verifies delegated
publisher targets, downloads bounded package archives, validates package identity and normalized
digest through `LocalPluginPackage`, and publishes an immutable local `PluginMarketplace`.
`RemotePluginMarketplaceConfig` is supplied by the trusted product composition root. Untrusted App
Server clients never supply URLs, roots, target names, archive paths, or cache paths.

The key private owners are:

- `MarketplaceTransport`: adapts the shared bounded HTTP client to TUF and rejects non-HTTPS
  network fetches;
- `metadata::published_plugins`: requires every package target to be signed by its exact
  `publishers/<publisher>` delegated role and validates target custom metadata;
- `archive::extract`: enforces archive/entry/expanded-size limits and rejects traversal, links,
  encryption, duplicate paths, and unsupported entries;
- `RemotePluginMarketplace::materialize`: verifies the revocation feed and every exact package
  before replacing the complete offline repository cache;
- `signed_repository_digest`: binds an immutable materialized snapshot to snapshot and targets
  metadata, including delegated role revisions;
- `replace_directory` / `recover_repository_cache`: keeps the previous complete repository until
  replacement succeeds and restores it after a crash between the two atomic renames.

Call flow:

```text
product-services.json + pinned root
  → RemotePluginMarketplace::sync
  → tough::RepositoryLoader (root rotation, threshold signatures, expiry, rollback)
  → delegated publisher metadata + top-level revocations
  → bounded ZIP extraction + LocalPluginPackage validation
  → immutable PluginMarketplace(RemoteManaged)
  → App Server PluginMarketplaceService
```

## Failure and offline semantics

- Signature, threshold, version, expiry, delegation, target hash/length, package digest, and archive
  failures are fail-closed.
- Only a transport failure may fall back to the last complete cache. Cached metadata is reopened
  through TUF with safe expiration enforcement; materialized package directories are not trusted
  alone.
- A remote cache is replaced only after all Zeta-specific package validation succeeds. Offline
  reads never rewrite the remote cache.
- TUF rollback metadata is copied into a staging datastore for each online refresh and committed
  only after package validation and complete repository caching succeed; an otherwise signed but
  unsafe package cannot poison the last-known-good rollback state.
- Revocations become durable exact-package tombstones in `zeta-plugins`; absence from a later feed
  does not silently restore activation.

## Extension and drift signals

New distribution protocols belong beside, not inside, `zeta-plugins` authority. Adding raw URL or
filesystem-path install RPCs, trusting a cached catalog without TUF reopen, treating a publisher
role as global revocation authority, or exposing Plugin credentials here would be architectural
drift.

The crate does not own Marketplace search/payment UX, Plugin activation/grants, Connector OAuth,
MCP sessions, or secret persistence.
