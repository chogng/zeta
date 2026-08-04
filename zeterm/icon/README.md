# `zeta-icon`

`zeta-icon` defines the renderer-independent icon asset contract consumed by
the reusable `zui` framework and `zeta-ui` components. It does not provide a
semantic icon catalog, product artwork, layout, theme, or GPU implementation.

## Ownership

| Concern | Owner |
| --- | --- |
| Stable asset identity and validation | `IconId` |
| Embedded artwork and rendering mode | `IconDefinition` / `IconRendering` |
| Copyable icon asset value | `Icon` |
| Product semantic icon catalog | An application crate such as `zeta-icons` |
| Icon placement and scene primitive | `zui::PaintIcon` |
| Rasterization, atlas, and GPU submission | Renderer backend |

Applications may define their own `Icon` values without depending on the
`zeterm` product icon catalog. `zeta-icons` is one optional catalog that maps
Zeta semantic icon identities to embedded SVG artwork.

## Verification

```bash
cargo test -p zeta-icon
```
