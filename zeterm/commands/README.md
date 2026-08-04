# zeta-commands

`zeta-commands` owns the stable product command vocabulary and the runtime-free
registration contract for the native `zeterm` host. Pointer activation, menu
items, keyboard shortcuts, and future command-palette entries converge on a
[`CommandRequest`](src/request.rs) before the host executes a state transition.

## Ownership

| Concern | Owner |
| --- | --- |
| Type-safe command identity | `ZetermCommandId` |
| Complete product command catalog | `ZetermCommandId::ALL` |
| Persisted command ID and shortcut label | `ZetermCommandId::id` / `ZetermCommandId::label` |
| Currently user-bindable command set | `ZetermCommandId::BINDABLE` |
| ID parsing and user-binding validation | `ZetermCommandId::from_id` / `ZetermCommandId::bindable_from_id` |
| Cross-module execution request | `CommandRequest` |
| Generic handler registration and lookup | `CommandRegistry<Context>` |
| `ElementId` mapping | `zeterm` host adapter |
| Keybinding context and platform defaults | `zeterm/src/keybindings.rs` |
| Built-in handler registration and product state | `zeterm/src/command_dispatch.rs` |

This crate must remain independent of `NativeApp`, `zui`, `zeta-winit`,
terminal state, workspace state, and the keybinding runtime. A command request
is a stable product intent; it is not a domain state machine or a replacement
for typed Session, Editor, Terminal, or App Server APIs.

## Execution path

```text
pointer / menu / shortcut / command palette
  → CommandRequest(ZetermCommandId)
  → CommandRegistry<NativeApp>
  → registered host handler
  → product state owner
  → invalidation / presentation rebuild
```

`CommandRegistry<Context>` owns only command-to-handler registration and lookup.
The registry is process-local to the host instance; it is not a global backend
registry. `zeterm` registers every built-in command during `NativeApp` startup,
while each handler remains responsible for calling the domain state owner.
`CommandRequest` currently carries only the command identity. A command that
needs parameters should add a typed request payload when that command is
introduced instead of passing UI element IDs or untyped service calls.

The string IDs are retained at the configuration boundary for existing
keybinding resources, including the current `workbench.action.*` compatibility
names. Internal callers use the enum so command consumers do not depend on
spelling or on UI element identity. A future namespace migration must add an
alias/compatibility path before changing persisted IDs.

## Verification

```bash
cargo test -p zeta-commands
```
