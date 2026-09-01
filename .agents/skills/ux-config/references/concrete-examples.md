# UX Config Concrete Examples

Use these examples only after deciding the owner, data kind, locality, and scope. They demonstrate the canonical target shapes; they do not prove that every migration or scope is already implemented. Before copying an example, verify its keys, allowed values, and scopes against the owning registry or schema.

## TypeScript profile settings

The TypeScript target is a plain `settings.json` object parsed as JSONC. It does not use a `version` and `values` wrapper:

```jsonc
{
  // Profile setting interpreted by the TypeScript UI.
  "editor.fontSize": 15,
  "editor.tabSize": 2,
  "workbench.colorTheme": "zeta-aurora"
}
```

This shape is valid only when each key is registered for the selected target. A key that supports profile scope does not automatically support workspace, workspace-folder, remote, or language scope. A `[typescript]` block means a language-specific override for files whose language identifier is `typescript`; it is unrelated to the UI being implemented in TypeScript and should appear only when that override is intentional.

## Rust GUI and TUI settings

Bounded Rust UI preferences share profile `config.toml` while remaining independently owned:

```toml
[gui]
theme = "zeta-aurora"
interfaceFontFamily = "sans-serif"
interfaceFontSize = 13
editorFontFamily = "monospace"
editorFontSize = 14
editorLineHeight = 21

[tui]
theme = "zeta-code-dark"
mouseInteractions = true
followUpMode = "queue"
inputMode = "standard"
statusLine = ["permissions", "model", "git-branch"]

[[tui.keybindings]]
key = "ctrl+y"
command = "zetaCode.action.copyLastResponse"
when = "inputFocus"
```

The GUI validates only the fields it owns under `[gui]`; the TUI does the same under `[tui]`. An editor must preserve unknown sibling keys when replacing either complete table. Directory permissions do not belong in either table.

## Theme selection and theme contents

The GUI example above persists only the selected identity `zeta-aurora`. The resource contents have a separate lifecycle under `themes/zeta-aurora.json`:

```json
{
  "$schema": "https://zeta.dev/schemas/color-theme.schema.json",
  "version": 1,
  "id": "zeta-aurora",
  "label": "Zeta Aurora",
  "colorScheme": "dark",
  "colors": {
    "workbench.background": "#0b1020",
    "editor.foreground": "#dbe7ff",
    "focusBorder": "#7aa2f7"
  }
}
```

Changing the selection must not rewrite the resource, and editing the resource must not create another persisted selection.

## API key entered through a UI

The API key is not JSON or TOML configuration. The provider credential owner stores opaque bytes under its secret key:

```text
Secret Store
provider/openai/default/api-key -> <opaque API key bytes>
```

Do not write the value to `settings.json`, `config.toml`, logs, state, or migration receipts. A UI may display whether the key is configured, but it must not read the stored value back.

## Session directory versus durable permission

Adding a directory uses the Session method and does not persist the directory or permissions:

```json
{
  "method": "session/dirs/add",
  "params": {
    "sessionId": "session-demo",
    "path": "/Users/example/project-docs",
    "permissions": ["readFiles", "searchFiles"]
  }
}
```

Remembering permissions is a separate explicit user action in the permission domain:

```json
{
  "method": "config/dirPermissions/set",
  "params": {
    "commandId": "remember-project-docs",
    "expectedRevision": 12,
    "path": "/Users/example/project-docs",
    "permissions": ["readFiles", "searchFiles"]
  }
}
```

The permission owner resolves the path to a stable directory identity before persistence. A directory config cannot issue the second request on its own, and `add-dir` must not issue it implicitly.

## One-way TypeScript settings migration

Legacy `configuration.json` may contain a wrapper:

```json
{
  "version": 1,
  "values": {
    "editor.fontSize": 15,
    "workbench.colorTheme": "zeta-aurora"
  }
}
```

The target `settings.json` contains only registered settings:

```jsonc
{
  "editor.fontSize": 15,
  "workbench.colorTheme": "zeta-aurora"
}
```

Validate the complete target, write it atomically, remove the legacy source only after durability is confirmed, and then stop reading the wrapper.
