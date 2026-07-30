# Product resources

This directory owns product resources that are shared across Zeta clients or
need a renderer-independent source of truth.

## Icons

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

The generated module exposes one named factory per SVG, and the watcher
regenerates it after an SVG is added, changed, or removed. `lxiconsLibrary`
registers only artwork that product code actually uses. This explicit semantic
registry boundary lets Renderer builds remove unused SVG factories; registering
the complete asset catalog eagerly would pull every icon into the bundle.

Use these commands from `desktop/`:

| Command | Purpose |
| --- | --- |
| `pnpm icons:generate` | Regenerate factories without rewriting source SVGs |
| `pnpm icons:watch` | Regenerate after files are added, changed, or removed |
| `pnpm icons:check` | Fail when source SVGs are not in canonical optimized form |
| `pnpm icons:optimize` | Optimize source SVGs and regenerate factories |
| `pnpm test:icons` | Test generation, optimization, deletion, and safety checks |

The normal `dev` and `dev:renderer` commands already run the icon watcher.
Prebuild also regenerates the module, so a stale generated file cannot reach a
production build.

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

Native currently embeds its existing local SVG asset. Moving Native onto this
canonical directory remains a separate integration change.
