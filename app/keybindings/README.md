# `zeta-keybindings-host`

`zeta-keybindings-host` owns the reusable application-host layer around the generic
[`zeta-keybinding`](../../zeta-rs/keybinding/README.md) model. It standardizes platform key
events, owns chord timeout state, and validates/polls the JSON user binding
resource without knowing a product's commands or window lifecycle.

The product host supplies a [`KeybindingCatalog`](src/catalog.rs) implementation
for command identity, builtin bindings, context conditions, and context lookup.
The host remains responsible for translating a resolved command into product
side effects, focus changes, IME lifecycle, and redraws.

## Ownership

| Concern | Owner | Boundary |
| --- | --- | --- |
| Logical/physical event normalization | `input` | Consumes public `zui` platform events and returns generic keybinding values. |
| Pending chord, timeout, blocker, and command resolution | `Keybindings<C>` | Uses the host-supplied catalog; does not execute commands. |
| User resource file size, polling and atomic writes | `KeybindingsResource<C>` | Owns one JSON file and preserves the last valid rule set on rejected updates. |
| User JSON shape, platform override and duplicate validation | `zeta-keybinding::user` | Compiles in-memory bytes through host command/condition callbacks; performs no file I/O. |
| Product command identity and builtin rules | Product host catalog | Implements `KeybindingCatalog`; product commands do not enter this crate's transport boundary. |
| Command execution, focus, IME, and event-loop deadlines | Product host | Consumes resolution and deadline accessors. |

## Execution path

```text
zui platform event
  → recording_chord / Keybindings::resolve
  → KeybindingCatalog context predicate
  → NoMatch / Pending / Command / Blocked
  → product host command executor

keybindings.json
  → KeybindingsResource::poll
  → zeta-keybinding::compile_user_bindings
  → catalog command/condition callbacks
  → complete UserBinding set
  → Keybindings::replace_user_bindings
```

`KeybindingsResource::poll` reads at most 1 MiB and accepts at most 1,024
entries. A malformed or oversized update returns `Rejected` and leaves the
engine's previous complete rule set in place. Writing a recorded binding
replaces only the same command's existing user rule and uses the shared
atomic-write helper.

## Verification

```bash
cargo test -p zeta-keybindings-host
cargo test -p app
```

The crate must not depend on `NativeApp`, App Server clients, terminal state,
workspace state, or product UI components. If it needs one of those types,
extend the host catalog contract or keep the adapter in `app/src`.
