# `zeta-composer`

`zeta-composer` owns the product-level geometry contract for zeterm's command composer. It
resolves the bottom-mounted composer panel, its fixed information/editor/toolbar rows, the
optional interaction list, and fixed-height interaction-list measurement. The crate is a pure
projection layer: it receives logical bounds and item counts, then returns geometry or generic
`zeta-ui` scroll commands.

## Boundary

| Capability | Owner | Status |
| --- | --- | --- |
| Composer panel and interaction-list geometry | `zeta-composer` | ✅ |
| Composer text, history, mode and selected item state | `zeterm/src/agent_composer.rs` and related host modules | ✅ host-owned |
| Code-editor document and text-input behavior | `zeterm/src/composer_editor.rs` | ✅ host-owned |
| Interaction item projection and activation | `zeterm/src/composer_interaction.rs` | ✅ host-owned |
| Scene painting, accessibility registration and product command mapping | `zeterm/src/composer_panel.rs` and host modules | ✅ host-owned |
| Renderer, window and platform event APIs | lower framework/backend and host adapters | ❌ |

The dependency direction is:

```text
zeterm host → zeta-composer → zeta-ui → zui
```

`zeta-composer` must not import `NativeApp`, workspace/session state, `winit`, `wgpu`, renderer
resources, or product icon catalogs. If a change needs those values, keep the adapter in
`zeterm/src` and pass only the resulting preferred size, item count, or logical bounds into this
crate.

## Execution path

`ShellLayout` asks `ComposerPanelLayout::for_main` for panel geometry. The host then paints and
registers the returned regions. Composer selection and wheel routing use
`interaction_list_bounds`, `interaction_content_size`, and
`interaction_selection_scroll_command` while the host retains `ScrollState` and applies the
resulting command.

Changes to panel spacing, minimum output height, interaction row sizing, or list visibility must
update `src/lib_tests.rs` and the native host tests that assert the final scene. Verify the crate
with:

```text
cargo test -p zeta-composer
```
