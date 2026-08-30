# zeta-keybindings-host

`zeta-keybindings-host` owns the reusable application-host layer around the generic [`zeta-keybinding`](../../zeta-rs/keybinding/README.md) model.

## Responsibilities

- `input` normalizes public `zui` key events, while `Keybindings<C>` owns pending chords, timeout state, blockers, and command resolution without executing commands.
- `settings` validates a product-owned configuration value and edits one command rule without reading or writing files.
- The Workbench adapter supplies `AppCommandId`, builtin rules, conditions, and context lookup; Workbench owns `[gui].keybindings`, command execution, focus, deadlines, and redraws.

## Execution path

```text
zui key event
  → Keybindings::resolve
  → KeybindingCatalog context predicate
  → NoMatch / Consumed / Command(AppCommandId)
  → WorkbenchApplication::dispatch_command

[gui].keybindings
  → zeta-keybinding::compile_user_bindings
  → complete UserBinding set
  → Keybindings::replace_user_bindings
```

The crate has no profile path, file watcher, or App Server dependency. Workbench obtains the opaque `[gui]` table from its current App Server connection and replaces active rules only after the whole keybinding value compiles.

## Verification

```bash
just test zeta-keybindings-host
just test zeta-workbench
```
