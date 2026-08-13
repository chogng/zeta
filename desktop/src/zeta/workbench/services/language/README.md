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
| `editor/contrib/folding/browser/` | Editor contribution | Folding range providers, tracked fold state, commands, and browser projection; it consumes language configuration but does not own language infrastructure |
| `languageCompletionSession*`, `languageDiagnostic*`, `languageTokenLineIndex.ts` | Editor language layer | Version gates, session state, and browser-facing result projection |
| `*Provider*`, `*Worker*`, `*Wire*` | Language contracts/runtime for now | Provider protocol and Worker transport; external adapters enter through this service instead of importing editor internals |

The editor language layer still owns the contracts and editor semantics consumed by those
providers: lexical fallback, bracket and pair editing, folding state, completion
sessions/snippets, result version gates, and browser presentation. This service
does not move those responsibilities into Workbench.

## LSP boundary

The LSP client and server lifecycle live below the Renderer in
`zeta-rs/lsp`, `zeta-rs/language-service`, and the App Server. This Workbench
service owns only the frontend adapters in `browser/appServerLanguageProviders.ts`
and `browser/appServerLanguageDiagnosticsService.ts`. The provider adapter exposes
hover, completion, cross-file navigation, call/type hierarchy, workspace symbols,
rename, code actions, document/range formatting, parameter hints, inlay hints,
and linked editing through
editor-owned contracts. The diagnostics adapter
reference-counts open models, debounces authoritative document synchronization,
closes the App Server document after the final editor releases it, and aggregates
push diagnostics with editor-published parser diagnostics by resource and revision.

The editor merges current-revision LSP diagnostics with parser diagnostics in its
existing decoration collection; stale revisions are hidden immediately after an
edit. The Workbench-owned Problems panel enumerates the complete shared workspace
repository, groups diagnostics by resource, filters by severity/message/file,
and delegates row navigation to `IEditorPart`; it does not inspect editor DOM or
own diagnostic production. The current App Server push-diagnostic producer only
synchronizes open models, while other producers may publish any workspace
resource. Pull/workspace diagnostics and semantic-token LSP adaptation remain
future work. Formatting edits use the
editor command/undo layer; parameter hints retain provider-selected active
signatures and parameters; inlay hints remain non-mutating; linked ranges extend
native input before commit so every synchronized change is one atomic undo step.
Regardless of origin, revision gates, application semantics, and DOM projection
stay editor-owned.

TextMate is a separate provider under `workbench/services/textMate`. The local
lexical provider remains the deterministic fallback when no external
provider is available.
