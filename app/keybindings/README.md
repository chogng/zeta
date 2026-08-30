# zeta-keybindings-host

`zeta-keybindings-host` owns the reusable application-host layer around the generic [`zeta-keybinding`](../../zeta-rs/keybinding/README.md) model.

## Responsibilities

- `input` normalizes public `zui` key events, while `Keybindings<C>` owns pending chords, timeout state, blockers, and command resolution without executing commands.
- `KeybindingsResource<C>` validates and polls one bounded JSON resource, preserves the last valid binding set after rejected updates, and writes recorded bindings atomically.
- The Workbench adapter implements `KeybindingCatalog` with `AppCommandId`, builtin rules, conditions, and context lookup; Workbench remains responsible for command execution, focus, input-method lifecycle, deadlines, and redraws.

## Execution path

```text
zui key event
  → Keybindings::resolve
  → KeybindingCatalog context predicate
  → NoMatch / Consumed / Command(AppCommandId)
  → WorkbenchApplication::dispatch_command

keybindings.json
  → KeybindingsResource::poll
  → zeta-keybinding::compile_user_bindings
  → complete UserBinding set
  → Keybindings::replace_user_bindings
```

`KeybindingsResource::poll` reads at most 1 MiB and accepts at most 1,024 entries. A malformed or oversized update returns `Rejected` without changing the active rules.

## Verification

```bash
just test zeta-keybindings-host
just test zeta-workbench
```
