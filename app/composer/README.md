# `zeta-composer`

`zeta-composer` owns app's Composer-local state and presentation contracts: multiline input,
automatic Agent/Shell routing, Shell history and completion, Slash/model-picker interaction,
interaction scroll state, panel geometry, and list measurement. `Composer` is the single state
owner; the app host adapts product state and executes returned submissions or model selections.

## Boundary

| Capability | Owner | Status |
| --- | --- | --- |
| Composer text, classified route, history and completion | `zeta-composer::Composer` | ✅ |
| Multiline document, selection, IME, ghost text and route-selected syntax | `zeta-composer::ComposerInput` over `zeta-editor` | ✅ |
| Slash/model-picker state and interaction scrolling | `zeta-composer::Composer` | ✅ |
| Composer panel and interaction-list geometry | `zeta-composer` | ✅ |
| Thread/history and App Server catalog adaptation | `app/src/composer_host.rs` | ✅ host-owned |
| Submission/model-selection side effects | `app` host | ✅ host-owned |
| Scene painting, accessibility registration and workspace toolbar | `app/src/composer_panel.rs` | ✅ host-owned |
| Renderer, window and platform event APIs | lower framework/backend and host adapters | ❌ |

The dependency direction is:

```text
app host → zeta-composer → zeta-editor / zeta-input-classifier / zeta-slash-commands
                             → zeta-ui → zui
```

`zeta-composer` must not import `NativeApp`, workspace/session state, `winit`, `wgpu`, renderer
resources, product icon catalogs, Thread state, or App Server transport DTOs. Normalize those
values in `composer_host.rs` before passing them into the crate.

## Execution path

Platform input mutates `ComposerInput`; `Composer` gives its text to `zeta-input-classifier` and
then selects `PlainText` or `Shell` presentation for the entire submission. Command-looking
prefixes are not highlighted independently: mixed input classified as an Agent message stays
plain text, while an input classified and submitted wholly as Shell uses Shell syntax.

`ShellLayout` asks `ComposerPanelLayout::for_main` for panel geometry. The host then paints and
registers the returned regions. Composer selection and wheel routing use
`interaction_list_bounds`, `interaction_content_size`, and
`interaction_selection_scroll_command`; the resulting scroll state remains inside `Composer`.

Changes to panel spacing, minimum output height, interaction row sizing, or list visibility must
update `src/lib_tests.rs` and the native host tests that assert the final scene. Verify the crate
with:

```text
cargo test -p zeta-composer
```
