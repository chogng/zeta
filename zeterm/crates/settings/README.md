# `zeta-settings`

`zeta-settings` owns the product Settings page shell: the left navigation,
titlebar-height header with search and close controls, action bar, and the
content slot for the active settings section. Configuration persistence and
native window integration stay outside this crate.

## Ownership

| Concern | Owner | Boundary |
| --- | --- | --- |
| Settings page rail, titlebar-height header, action bar, and content geometry | `SettingsPageLayout` / `SettingsPage` | Owns page composition; the host supplies the header height and it does not know the native window or config transport. |
| Search, close, navigation, and page action element identities | `zeta-settings` | Publishes stable interaction nodes and a modal focus boundary. |
| Back item and semantic icon+label navigation | `SettingsPage` | Keeps Settings navigation presentation and activation identities together; the host maps Back to its surface transition. |
| Active section content | Settings section host | Paints into `SettingsPage::content_bounds()`. |
| Generic controls and layout primitives | `zeta-ui` / `zui` | Supplies reusable presentation and backend-neutral interaction contracts. |
| Configuration load/save and platform events | Native/config host | Adapts DTOs, executes actions, and owns IME/window lifecycle. |

## Execution path

```text
native snapshot → SettingsPage shell + active section → UiIntent::Activate
                → native/config adapter executes the action
```

The first active section is Language Servers. Its existing native model and
content view are mounted into the page content slot while the page shell is
owned by this crate. The header close control and Back item both remain
page-owned interactions; the host decides how they close or leave Settings.
New sections should add their own product state and content composition here
rather than growing `native` with another page layout.

## Verification

Run `cargo test -p zeta-settings` and `cargo test --manifest-path Cargo.toml -p zeterm` after changing
the page contract or a host adapter.
