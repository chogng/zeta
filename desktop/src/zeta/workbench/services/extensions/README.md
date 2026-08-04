# Declarative extension resources

This service is the Workbench composition boundary for static extension packages. Rust owns
discovery and filesystem authority; the renderer receives validated manifest data and explicitly
requested resource bytes; Workbench decides which declarative contributions become active.
Extensions do not receive access to the editor model, editor DOM, Worker ports, or arbitrary
TypeScript execution.

## Ownership

| Area | Owner | Current contract |
| --- | --- | --- |
| Trusted package roots and `package.json` discovery | `zeta-extensions::ExtensionCatalog` | Built-in and user roots; direct child packages only |
| Manifest identity, resource containment, size limits, and SHA-256 | `zeta-extensions` | Invalid packages produce diagnostics; unsafe resource paths are rejected |
| Renderer transport | `platform/extensions/*` | `IExtensionApi.list` and `IExtensionApi.readResource` |
| Manifest contribution parsing | `parseExtensionManifest` | Validates identity and supported declarative fields |
| Workbench lifecycle and registration | `AppServerExtensionService` | Loads a fresh catalog transactionally and reports failures |
| TextMate grammar registry and Worker runtime | `workbench/services/textMate` | Receives loaders; never reads extension paths itself |
| Editor semantics and presentation | Editor runtime | Consumes token results; never discovers extension packages |

## TextMate resources supplied by an extension

The first declarative slice is `contributes.grammars`. The extension package supplies:

| Manifest field | Meaning | Consumer |
| --- | --- | --- |
| `contributes.grammars[].path` | Relative path to the raw JSON or plist grammar file | Rust resource store, then TextMate loader |
| `scopeName` | Root grammar scope identity | TextMate registry |
| `language` | Root grammar language ID | TextMate provider selection |
| `injectTo` | Root scopes receiving an injection grammar | TextMate registry |

`package.json` itself is also extension-owned resource metadata, but Rust returns its validated
descriptor and manifest JSON rather than letting the renderer open an arbitrary manifest path.
Grammar files are read as bounded UTF-8 data and are materialized into the existing versioned
TextMate catalog before the dedicated Worker sees them.

The following TextMate contribution fields are not silently ignored. They currently fail that
extension activation and remain an explicit next step: `embeddedLanguages`, `tokenTypes`,
`balancedBracketScopes`, and `unbalancedBracketScopes`. Supporting them requires extending the
frontend grammar contract, the transferable catalog, and editor token projection together.

## Execution path

```text
trusted extension roots
  -> Rust ExtensionCatalog
  -> extensions/list + extensions/resource/open
  -> Electron/Vite IExtensionApi
  -> AppServerExtensionService
  -> TextMateGrammarService
  -> versioned grammar catalog
  -> dedicated TextMate Worker
```

The service replaces registrations only after every manifest and registration in the new catalog
has succeeded. A malformed package or unreadable grammar leaves the previous active catalog in
place and emits `onDidFail`; it does not activate a partial extension.

## Deliberate non-goals

This boundary does not execute extension JavaScript, load arbitrary URLs, treat a workspace folder
as a trusted extension root, or move TextMate parsing into Rust. Rust supplies authoritative bytes
and path validation; the browser-owned TextMate runtime remains responsible for parsing, injections,
scope resolution, incremental tokenization, and Worker transport.

## Current status and next steps

Current: built-in/user static package discovery, manifest identity validation, safe resource reads,
TextMate root/injection grammar registration, and explicit failure reporting are implemented.

Next: add declarative language associations/configuration, then decide whether embedded-language and
bracket metadata should be represented in the editor token contract. Runtime extension hosts and
arbitrary extension code are separate trust boundaries and are not implied by this service.
