# zeta-marketplace-client

`zeta-marketplace-client` owns Zeta's remote Marketplace adapter and the contracts on both sides of
the local Manager. It is the only Zeta crate that understands the current Marketplace HTTPS/TUF
static distribution.

Canonical cross-crate ownership is documented in
[Marketplace integration](../../docs/marketplace-integration.md). This README owns the crate's exact
implementation contract.

## Ownership

| Surface | Owner |
| --- | --- |
| product-facing package/install/capability DTOs | `model.rs` + `MarketplaceServiceClient` |
| remote search/get/download port | `MarketplaceRegistryClient` |
| signed catalog resolution | private `Catalog` |
| TUF refresh, trust, revocation and target verification | private `RemoteSource` |
| ZIP extraction and remote cache | private `archive` / `remote::cache` |
| local artifact store, installations and leases | ❌ `zeta-marketplace-manager` |
| capability authorization and activation | ❌ product runtimes |

The current remote implementation is an adapter, not a product API. App Server and Renderer never
receive catalog manifests, target names, URLs, archives, cache paths or extracted paths. If the
remote Marketplace later exposes a true HTTPS business service, only the implementation behind
`MarketplaceRegistryClient` needs to change.

## Call path

```text
MarketplaceManager
  → MarketplaceRegistryClient
      → MarketplaceRemoteClient
          → Catalog
              → RemoteSource
                  → HTTPS/TUF static distribution
```

`MarketplaceRemoteClient::open` is network-free so Marketplace availability cannot block App Server
startup. The first Marketplace operation lazily loads the signed catalog and later failed initial
loads remain retryable. `search` and `get` reuse the in-process verified snapshot until the
product-selected `catalog_refresh_interval` elapses, then synchronously refresh it through TUF.
`download` always refreshes trust metadata, checks exact revocation state, downloads and
verifies the TUF target, extracts it into private temporary storage, and returns only a
`MarketplacePackagePayload` object. The local Manager may copy that payload into an empty staging
directory but cannot discover the remote cache or extraction path.

Schema 2 MCP records may carry a signed `upstream` pointer to the exact official MCP Registry
record used by the Marketplace publisher. The client validates and projects that provenance for
display, while installation still uses only the TUF-authenticated Marketplace target. An upstream
link is never treated as an executable or download URL.

## Key symbols

| Symbol | Contract |
| --- | --- |
| `RemoteMarketplaceConfig` | product-pinned HTTPS metadata/target URLs, trusted root, cache root, publisher policy and bounded catalog refresh interval |
| `MarketplaceRemoteClient` | concrete current remote distribution adapter |
| `MarketplaceRegistryClient` | narrow remote discovery/download dependency injected into the local Manager |
| `MarketplacePackagePayload` | opaque verified handoff; authorizations copy-to-staging, never path access |
| `MarketplaceServiceClient` | complete product-facing service implemented by the local Manager |
| `MarketplaceClientError` | stable business/unavailable/protocol error categories without private diagnostics |

Architectural drift includes exporting TUF/catalog/archive types, adding paths or URLs to public
DTOs, letting App Server parse distribution metadata, or moving local installation state into this
crate.

## Failure semantics

- trust, digest, revocation, archive and signed-catalog failures fail closed as `packageUntrusted`;
- missing package/version returns stable business errors;
- remote or cache availability failures do not reveal private filesystem diagnostics;
- an existing verified TUF cache may be used only through the fail-closed cache policy.

## Verification

```bash
cargo test -p zeta-marketplace-client
cargo clippy -p zeta-marketplace-client --all-targets -- -D warnings
```

Current limitations: discovery refresh is synchronous and the supported remote transport is the
static TUF distribution. Plugin and Language package lifecycle now use this service; capability
authorization and execution remain domain-runtime responsibilities.
