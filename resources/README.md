# Product resources

This directory owns product resources that are shared across Zeta clients or
need a renderer-independent source of truth.

## Product services

`product-services/` is the release-owned trust bundle copied to
`zeta-resources/product-services/` by both package assemblers. Its
`product-services.json` registers the product-selected HTTPS Marketplace registry and references the sibling
`marketplace-root.json`; the root is public verification material, never a signing key.

Packaged Desktop/server hosts, `zeta code`/TUI, and app discover this file through the shared
App Server client + `zeta-install-context` boundary. Each host explicitly injects the typed result;
an explicit `ZETA_PRODUCT_SERVICES_PATH` remains authoritative for development and specialized hosts.
Marketplace URLs or root replacement must not move into user configuration or Plugin metadata.

The independent Marketplace source, public root owner, publishing pipeline, and key rotation
procedure live in the private [`marketplace`](https://github.com/chogng/marketplace) repository.
Zeta is one optional consumer: this product bundle chooses to pin that root, while Marketplace
validation and publication do not depend on Zeta. A root rotation must be valid in the Marketplace;
Zeta then updates its pinned copy before requiring metadata signed only by the rotated root.

## Icons

The cross-client ownership and rendering contract is documented in
[`docs/icons.md`](../docs/icons.md).

`icons/` contains Zeta's canonical, first-party SVG artwork imported from the
local `lxicons` working repository.

- These files are product UI artwork. The Seti file-icon theme remains owned by
  `zeta-rs/file-icons` and is intentionally not duplicated here.
- Consumers should bind artwork to Zeta's semantic icon IDs instead of exposing
  filenames or raw SVG markup through product interfaces.
- Renderer-specific loading, optimization, tinting, caching, and rasterization
  belong to each client adapter.

## Desktop integration

Desktop consumes these files through the following build-time path:

```text
resources/icons/*.svg
  -> build/desktop/resources/syncProductIcons.ts
  -> zeta-ts/generated/product-icons.ts
  -> base/common/lxiconsLibrary.ts
  -> register
  -> browser appendIcon
```

The generated module exposes one named factory per SVG. The normal package
lifecycle synchronizes it before development and builds, while the Vite plugin
keeps it synchronized after an SVG is added, changed, or removed.
`lxiconsLibrary` registers only artwork that product code actually uses. This
explicit semantic registry boundary lets Renderer builds remove unused SVG
factories; registering the complete asset catalog eagerly would pull every icon
into the bundle.

Use these commands from `zeta-ts/`:

| Command | Purpose |
| --- | --- |
| `pnpm icons:sync` | Canonicalize source SVGs and synchronize generated factories |
| `pnpm icons:check` | Fail when source SVGs are not in canonical optimized form |
| `pnpm test:icons` | Test generation, optimization, deletion, and safety checks |

The normal `dev` and `dev:renderer` commands activate the Vite integration.
Prebuild also synchronizes the module, so a stale generated file cannot reach a
production build. Synchronization writes source and generated files only when
their canonical content changes.

Optimization uses SVGO with multiple passes. It removes editor metadata,
comments, redundant groups and attributes, minifies path data, removes fixed
root dimensions while preserving `viewBox`, and gives IDs an icon-specific
prefix so multiple inline SVGs cannot collide in one document. Generation also
rejects malformed filenames, multiple SVG roots, scripts, `foreignObject`,
event handlers, and linked content before markup reaches the browser parser.

The browser renderer parses and configures one prototype per icon definition
and `Document`, then deep-clones that detached prototype for each rendered
instance. This avoids repeated `innerHTML` parsing while keeping every rendered
SVG independently mutable and allowing documents to be garbage-collected.

## Rust integration

Rust consumes the same source artwork through:

```text
resources/icons/*.svg
  -> build/app/syncRustIcons.ts
  -> app/icons/src/generated.rs
  -> private artwork bindings
  -> app/icons/src/library.rs
  -> stable zeta-icons semantic library
  -> zeta-ui renderer and components
```

Run `node build/app/syncRustIcons.ts` after adding, removing, or renaming an
icon; `node build/app/syncRustIcons.ts --check` verifies that the checked-in
output is current. The generated Rust source is checked in so Cargo and Bazel
builds remain hermetic. Generated artwork is crate-private: resource filenames
do not automatically become public icon IDs. Add or change public semantics in
`app/icons/src/library.rs`. `zeta-icons` classifies fixed non-symbolic colors
as multicolor; `zeta-ui` preserves those colors in an sRGB atlas while routing
black symbolic coverage through the caller-tinted mask atlas.
