# zeta-plugin-marketplace

`zeta-plugin-marketplace` owns verified remote distribution for product-managed Plugin
Marketplaces. The cross-crate Plugin/Connector/MCP product model is canonical in
[`docs/plugins.md`](../../docs/plugins.md); this README documents the exact implementation
contract of this crate.

## Ownership

`RemotePluginMarketplace` refreshes a TUF repository with a host-pinned root, verifies delegated
publisher targets, projects signed discovery metadata into an immutable `PluginMarketplace`, and
defers bounded package download until an exact install or update requests the bytes.
`RemotePluginMarketplaceConfig` is supplied by the trusted product composition root. Its transport
mode is always remote, while `PluginMarketplaceTrust` separately distinguishes a product-managed
source from a host-approved external source. Untrusted App Server clients never supply URLs, roots,
target names, archive paths, cache paths, or trust labels.

The key private owners are:

- `MarketplaceTransport`: adapts the shared bounded HTTP client to TUF and rejects non-HTTPS
  network fetches;
- `metadata::published_plugins`: requires every package target to be signed by its exact
  `publishers/<publisher>` delegated role, preserves the backward-compatible `zetaPlugin` identity,
  and validates optional `zetaCatalog` manifest/statistics metadata;
- `archive::extract`: enforces archive/entry/expanded-size limits and rejects traversal, links,
  encryption, duplicate paths, and unsupported entries;
- `RemotePluginMarketplace::materialize` / `catalog_packages`: verify the revocation feed and build
  package descriptors without reading ZIP targets when signed discovery metadata is present;
- `RemotePackageMaterializer` / `materialize_package`: reopen current TUF authority at install time,
  reject newly revoked targets, download one exact ZIP, validate it through `LocalPluginPackage`, and
  promote it into the digest-keyed package cache;
- `signed_repository_digest`: binds an immutable catalog snapshot to snapshot and targets metadata,
  including delegated role revisions;
- `cache_repository`: retains verified metadata and the revocation target for offline browsing; it
  does not prefetch Plugin archives;
- `cache::prune`: bounds materialized package count and expanded bytes, evicts packages absent from
  the current non-revoked signed target set first, then oldest materializations, and never touches
  the separate installed content-addressed store;
- `CacheCoordinator` / `CacheMaterializationLease`: serialize cache mutations across distributor
  instances sharing one cache root and protect every digest still being copied into Plugin
  authority;
- `replace_directory` / `recover_complete_directory`: keeps the previous complete repository until
  replacement succeeds and restores it after a crash between the two atomic renames.

Call flow:

```mermaid
flowchart TD
    C["product-services.json + pinned root"] --> S["RemotePluginMarketplace::sync"]
    S --> T["TUF metadata, delegation, expiry, rollback"]
    T --> D["signed zetaCatalog + revocations"]
    D --> M["PluginMarketplace(RemoteManaged)"]
    M --> A["App Server discovery"]
    M -->|"exact install/update"| P["RemotePackageMaterializer"]
    P --> Z["one bounded ZIP + LocalPluginPackage validation"]
    Z --> Q["bounded digest materialization cache"]
    Q --> O["separate content-addressed Plugin authority store"]
```

## Failure and offline semantics

- Signature, threshold, version, expiry, delegation, target hash/length, package digest, and archive
  failures are fail-closed.
- Verified-external sources additionally require a host-pinned, non-empty publisher allowlist.
  Every signed target must remain inside that scope; remote metadata cannot broaden it.
- Only a transport failure may fall back to the last complete discovery cache. Cached metadata is
  reopened through TUF with safe expiration enforcement; package directories are never trusted
  without the current signed target and revocation metadata.
- Offline browsing works while cached metadata remains valid. Offline installation works only when
  that exact package was previously materialized; an uncached ZIP requires the distribution.
- A catalog refresh replaces metadata only after signed discovery metadata passes Zeta manifest,
  identity, contribution, and statistics validation. Legacy targets without `zetaCatalog` use a
  compatibility download during refresh until the publisher republishes richer metadata.
- TUF rollback metadata is copied into a staging datastore for each online refresh and committed
  only after discovery validation and repository caching succeed. Install-time refresh commits its
  rollback state only after the selected package passes archive and normalized-digest validation.
- Revocations become durable exact-package tombstones in `zeta-plugins`; absence from a later feed
  does not silently restore activation.
- The default materialization budget is 32 packages and 1 GiB expanded bytes per Marketplace;
  product composition may select a smaller bounded policy. Automatic reconciliation prioritizes
  evicting unlisted or revoked digests and reports retained, evicted, and soft-excess counts/bytes.
- The exact package currently being handed to install is protected until the authority has copied
  it. A package larger than the selected byte budget may therefore create temporary reported
  excess; the next unprotected sync or explicit `prune_cache` removes it. Installed objects are in
  another store and survive cache eviction.

## Extension and drift signals

New distribution protocols belong beside, not inside, `zeta-plugins` authority. Adding raw URL or
filesystem-path install RPCs, trusting a cached catalog without TUF reopen, treating a publisher
role as global revocation authority, or exposing Plugin credentials here would be architectural
drift.

The crate does not own Marketplace search/payment UX, Plugin activation/grants, Connector OAuth,
MCP sessions, secret persistence, download progress UI, or installed-object retention policy.
