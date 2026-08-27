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

## Application branding

The fixed application icon is derived from
`zeta-ts/src/zeta/workbench/browser/media/zeta-light.svg`. Unlike renderer UI,
launcher, package, taskbar, and Web icons do not change with the editor color theme.

- `win32/zeta.ico` is the Windows application and package icon.
- `darwin/zeta.icns` is the macOS application bundle icon.
- `linux/zeta.png` is the Linux desktop and window icon.
- `server/` contains the Web favicon, install icons, and manifest.

Vite copies `server/` unchanged to the renderer output root, and the browser
Workbench and Sessions pages link those stable paths. The repository does not
currently contain an Electron bundle or installer stage; that stage must consume
the three platform files directly when it is introduced.

`zeta-dark.svg` remains a renderer-only titlebar variant and is not a packaging
source.

## Icons

The cross-client ownership and rendering contract is documented in [`docs/icons.md`](../docs/icons.md).

`icons/*.svg` is the only hand-maintained input for Zeta product icons. Add, replace, or remove an SVG and run `pnpm icons:generate` from the repository root; `build/resources/icons/generate.ts` canonicalizes the SVG and generates `icons/manifest.json`, `zeta-ts/generated/product-icons.ts`, and `app/icons/src/generated.rs` together through `generate-to-ts.ts` and `generate-to-rs.ts`.

- SVG filenames use lowercase kebab-case and become the icon IDs without a second mapping table.
- `manifest.json` is generated output; do not edit its `file` or `rendering` fields.
- The Seti file-icon theme remains owned by `zeta-rs/file-icons` and is intentionally not duplicated here.
- Renderer-specific tinting, caching, rasterization, and component layout remain in each client.

The generator uses SVGO with multiple passes, removes fixed root dimensions while preserving `viewBox`, prefixes SVG IDs, rejects active or linked content, and infers `symbolic` or `multicolor` from the optimized paint values. `pnpm icons:check` verifies the SVGs and all generated outputs without modifying files; `pnpm test:icons` covers generation, optimization, deletion, safety checks, manifest metadata, and the Vite update path.
