# `zeta-settings`

`zeta-settings` owns the product Settings page shell: the left navigation, titlebar-height header with search and close controls, action bar, and the content slot for the active settings section. The page can be mounted as a workbench surface or as a modal host; configuration persistence and native window integration stay outside this crate.

## Ownership

| Concern | Owner | Boundary |
| --- | --- | --- |
| Settings page rail, titlebar-height header, action bar, and content geometry | `SettingsPageLayout` / `SettingsPage` | Owns page composition; the host supplies the header height and it does not know the native window or config transport. |
| Search, close, navigation, and page action element identities | `zeta-settings` | Publishes stable interaction nodes for every section; `SettingsPageMode` determines whether it creates a modal focus boundary. |
| Section selection and selected navigation semantics | `SettingsPageSection` / `SettingsPage` | Keeps the active section presentation contract in the shell; the host maps each navigation identity to its section state. |
| Back item and semantic icon+label navigation | `SettingsPage` | Keeps Settings navigation presentation and activation identities together; the host maps Back to its surface transition. |
| Active section content | Settings section host | Paints into `SettingsPage::content_bounds()`. |
| Generic controls and layout primitives | `zeta-ui` / `zui` | Supplies reusable presentation and backend-neutral interaction contracts. |
| Configuration load/save and platform events | Native/config host | Adapts DTOs, executes actions, and owns IME/window lifecycle. |

## Execution path

```text
native snapshot → SettingsPage shell + active section → UiIntent::Activate
                → native/config adapter executes the action
```

The page currently exposes General, Language Servers, Appearance, and Keybindings. Language Servers mounts its existing native model and content view; the other sections receive host-projected workspace, theme, and keybinding data, while their domain state and persistence remain outside this crate. The header close control and Back item remain page-owned interactions; the host decides how they leave the top-level Workbench item. Settings is selected by the host-owned Workbench navigator and rendered here as a center Settings Pane; `SettingsPageMode::Surface` keeps it inside the shared shell while Sessions and other sibling parts remain available. New sections should add their own product state and content composition here rather than growing `native` with another page layout.

## Verification

Run `cargo test -p zeta-settings` and `cargo test --manifest-path Cargo.toml -p app` after changing
the page contract or a host adapter.
