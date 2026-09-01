# Selecting an Artifact for Unlisted UX Data

Read this reference when persistent UX data is not covered by a host routing table, when its existing location is disputed, or when deciding whether to create a new resource.

## Find the semantic owner

Identify the code that interprets the value, validates the complete candidate, applies it at runtime, exposes user editing, and reports errors. That UI host is the semantic owner even when another process performs file I/O or revision checks.

Do not classify data as UX configuration merely because a settings screen edits it. Secrets, permissions, provider behavior, execution policy, and service behavior retain their own domain owners.

## Decide whether it is configuration

| Question | Decision |
| --- | --- |
| Does it express durable user intent that should survive restart? | Continue classification |
| Can it be reconstructed from the current window, session, workspace, or cached data? | Store it as state, not configuration |
| Does it grant capability, control backend behavior, or contain a secret? | Route it to the permission, backend, or secret owner |
| Does it select a named resource? | Store the selected identity as a setting; classify the resource contents separately |
| Is it a built-in default with no user choice? | Keep the default in code, not in persisted configuration |

## Worked classification examples

These examples demonstrate the decision process; they do not assert that a named setting, command, editor, or persistence path is already implemented.

| Request | Decision | Reason and required follow-up |
| --- | --- | --- |
| Persist the TypeScript editor's font size across restarts | Register a profile setting in `settings.json` | The TypeScript UI interprets a bounded durable preference; add only the scopes its semantics require |
| Let the Rust GUI select a theme | Store the selected identity under `[gui]` | Selection follows the GUI settings lifecycle; validate the referenced theme separately |
| Edit theme contents consumed by both graphical UIs | Use one shared schema-validated theme resource only when one registry owns the complete document | Multiple consumers do not justify duplicate files or competing validators; each UI still owns its selection |
| Rebind a Rust TUI application command | Use `[tui].keybindings` when it shares the TUI settings editor and revision | Validate the complete ordered rule set against the TUI command and context catalogs before applying it |
| Restore window bounds, active tabs, and scroll positions | Use the owning UI state store | These values describe reconstructable runtime state rather than durable user intent |
| Save an API key entered through a settings screen | Route the value to the secret owner | The editing surface does not make a credential UX configuration; settings may retain only a non-sensitive reference when the secret contract supports one |
| Add a directory for the current Session | Keep the directory and granted capabilities in Session state | `add-dir` must not create a durable permission entry or write a UI settings artifact |
| Remember a directory's capabilities for later Sessions | Use a separate explicit permission action and the user profile's permission domain | Persist the complete capability set by stable directory identity; the directory's own config cannot grant it |
| Change provider, execution, or service behavior | Route the value to the owning backend domain | A UI may edit the value, but it does not own interpretation, defaults, validation, or runtime application |

After making the classification, read [concrete-examples.md](concrete-examples.md) when an exact JSON, JSONC, TOML, secret, permission, or migration shape would clarify the implementation. Verify every copied key against its owning registry or schema.

## Choose the artifact lifecycle

| Data contract | Artifact |
| --- | --- |
| Bounded preference interpreted by one UI host and changed with that host's settings | TypeScript `settings.json` parsed as JSONC, Rust GUI `[gui]`, or Rust TUI `[tui]` |
| Bounded ordered value sharing the Rust host's settings editor and revision | A typed ordered value inside `[gui]` or `[tui]` |
| Ordered rules with an independent editor, complete-document replacement, and their own revision | A dedicated JSONC resource when comments are supported, otherwise strict JSON |
| Installable, exchangeable, or schema-versioned UX content | A dedicated strict JSON document, normally one document per resource |
| Multiple documents discovered from a directory | A host-owned resource directory with file count, size, identity, duplicate, and schema limits |

Complexity or size alone does not justify a new file. Split a resource when it needs an independent editor, revision, import/export boundary, distribution identity, schema lifecycle, or external-edit behavior.

## Choose the text format

| Requirement | Format |
| --- | --- |
| TypeScript settings with contributed keys, layered scopes, comments, and syntax-aware edits | JSONC |
| Bounded Rust host settings edited as one profile table | TOML |
| User-authored ordered resource whose parser and writer preserve comments | JSONC |
| Versioned exchange resource requiring deterministic strict validation | JSON |

The extension does not define parser behavior. `settings.json` deliberately uses a `.json` filename while accepting JSONC. State whether comments and trailing commas are accepted, whether writers preserve them, and whether unknown fields are rejected or retained.

## Choose scope and merge behavior

Default to the profile beside the interpreting UI process for a user interface preference. Add remote, workspace, workspace-folder, or language scope only when the value legitimately changes at that boundary and the interpreting host supports it. An active backend connection does not move a device or UI-process preference to that backend. Do not add scope because a nearby file happens to support it.

Define whether narrower scopes replace, merge, or append. Ordered rules require explicit precedence, duplicate identity, blocker, and conflict behavior; object settings require field ownership and unknown-field behavior. Do not invent a merge when complete replacement is the actual contract.

## Keep authority decisions outside UX settings

Opening a directory for the current Session and remembering permissions for later Sessions are different actions:

- `add-dir` adds a directory and an explicit capability set to the current Session only. It must not write `[tui]`, `[gui]`, TypeScript settings, or durable directory permissions.
- A separate explicit "remember these permissions" action may persist a complete capability set in the user profile's permission domain, keyed by the directory's stable identity. It must not use a display path as identity.
- A directory-provided config file cannot grant permissions to itself. Durable permission remains user-owned profile data.
- Do not model Zeta permissions as a `trusted` boolean. Preserve the complete capability set so each permission can be reviewed and revoked independently.

## Complete the resource contract

Before implementation, define:

- stable setting key or resource identity;
- default and allowed values;
- parser, serializer, schema version, and complete-candidate validation;
- ordering, duplicate, unknown-field, and size limits where applicable;
- revision, atomic write, external reload, and conflict behavior;
- runtime application owner and invalid-update behavior;
- legacy source, target, conflict handling, and migration removal boundary.

If the result uses an existing artifact class, add the durable mapping to the corresponding host reference only when it represents a reusable data family or a new boundary. If it introduces a genuinely independent resource kind, create one focused reference and link it from the host reference. Do not create one reference per key.
