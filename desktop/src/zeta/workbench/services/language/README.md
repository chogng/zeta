# Workbench language service

`workbench/services/language` owns the shared language-provider composition used
by editor products. `LanguageFeaturesService` owns the registration lifecycle for
language configuration, syntax providers, and completion providers, then
creates caller-owned per-document language services.

`registerLanguageConfiguration`, `registerSyntaxProvider`, and
`registerCompletionProvider` are the composition seam for extension-host, LSP,
or Rust-backed adapters. Registrations are disposable and remain independent of
per-document service lifetimes.

The filename split is intentional:

| Filename family | Owner | Responsibility |
| --- | --- | --- |
| `languageFeaturesService.ts` | Workbench | Shared registration and per-document service composition |
| `languageLexical*`, `languagePair*`, `languageBracket*` | Editor language layer | Deterministic editor semantics and editing behavior |
| `editor/alpha/contrib/folding/browser/` | Editor contribution | Folding range providers, tracked fold state, commands, and browser projection; it consumes language configuration but does not own language infrastructure |
| `languageCompletionSession*`, `languageDiagnostic*`, `languageTokenLineIndex.ts` | Editor language layer | Version gates, session state, and browser-facing result projection |
| `*Provider*`, `*Worker*`, `*Wire*` | Language contracts/runtime for now | Provider protocol and Worker transport; external adapters enter through this service instead of importing editor internals |

The editor language layer still owns the contracts and editor semantics consumed by those
providers: lexical fallback, bracket and pair editing, folding state, completion
sessions/snippets, result version gates, and browser presentation. This service
does not move those responsibilities into Workbench.

## LSP boundary

An LSP client is not implemented here yet. Future LSP/JSON-RPC and server
lifecycle code belongs in this Workbench service layer and should register thin
adapters against the language provider contracts. Diagnostics, completion, semantic
tokens, and provider-backed folding may originate from LSP; their versioned
application and DOM projection remain editor-owned.

TextMate is a separate provider under `workbench/services/textMate`. The local
The lexical provider remains the deterministic fallback when no external
provider is available.
