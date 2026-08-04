# Product resources

This directory owns product resources that are shared across Zeta clients or
need a renderer-independent source of truth.

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
  -> desktop/scripts/sync-product-icons.mjs
  -> desktop/generated/product-icons.ts
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

Use these commands from `desktop/`:

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
  -> scripts/sync-rust-icons.mjs
  -> zeterm/icons/src/generated.rs
  -> private artwork bindings
  -> zeterm/icons/src/library.rs
  -> stable zeta-icons semantic library
  -> zeta-ui renderer and components
```

Run `node scripts/sync-rust-icons.mjs` after adding, removing, or renaming an
icon; `node scripts/sync-rust-icons.mjs --check` verifies that the checked-in
output is current. The generated Rust source is checked in so Cargo and Bazel
builds remain hermetic. Generated artwork is crate-private: resource filenames
do not automatically become public icon IDs. Add or change public semantics in
`zeterm/icons/src/library.rs`. `zeta-icons` classifies fixed non-symbolic colors
as multicolor; `zeta-ui` preserves those colors in an sRGB atlas while routing
black symbolic coverage through the caller-tinted mask atlas.
