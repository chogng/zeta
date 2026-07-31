# Zeta TextMate adapter

`editor/textmate` adapts TextMate grammars to Alpha's versioned Analysis
provider contract. It is an editor-domain adapter, not part of `base`, and it
does not own workspace files, extension manifests, themes, documents, or the
Alpha view. Its browser boundary owns only explicitly bundled grammar assets.

## Ownership

| Capability | Owner | Status |
| --- | --- | --- |
| Grammar contribution identity and revision snapshots | `TextMateGrammarRegistry` | ✅ |
| Transferable grammar catalogs and materialization | `TextMateGrammarCatalogModel` / `materializeTextMateGrammarCatalog` | ✅ |
| Atomic Worker catalog state and side-channel transport | `TextMateGrammarCatalogStore` / catalog wire | ✅ |
| TextMate runtime and incremental line-state cache | `TextMateTokenizationService` | ✅ |
| Scope-to-Alpha token vocabulary mapping | `TextMateScopeResolver` | ✅, replaceable |
| Alpha Analysis provider/module adaptation | `createTextMateAnalysisProvider` / `createTextMateAnalysisModule` | ✅ |
| Catalog-gated Analysis Worker composition | `TextMateAnalysisModuleWorkerClient` / `browser/textMateAnalysisWorkerMain.ts` | ✅ |
| Browser Worker Oniguruma WASM loading | `browser/textMateOniguruma.ts` | ✅ |
| Grammar contribution-to-catalog lifecycle | `TextMateGrammarService` | ✅ |
| Product-bundled JSON/JSONC grammar resources | `BrowserTextMateGrammarService` | 部分具备 |
| External extension-manifest loading | future extension composition root | 尚未完成 |
| Theme-specific TextMate selector resolution | theme adapter above this module | 尚未完成 |

`editor/textmate/common` may depend on Alpha's public Analysis and text
contracts because it adapts into that domain. Alpha and `base` must not import
TextMate runtime types. `editor/textmate/browser` is the only layer that knows
the `onig.wasm` asset URL or uses `fetch`.

## Grammar contract

`TextMateGrammarRegistry.register` accepts a root grammar, an injection grammar,
or both:

- `scopeName` is the unique TextMate identity;
- an optional concrete `languageId` selects the one root grammar for that
  language;
- `injectTo` declares root scopes that should load the grammar as an injection;
- `loadGrammar` returns raw JSON/plist text or a parsed `IRawGrammar`.

Registrations are caller-owned and disposable. Every change publishes a new
immutable `TextMateGrammarRegistrySnapshot`; old snapshots remain internally
consistent. `materializeTextMateGrammarCatalog` resolves one snapshot into a
bounded, structured-clone-safe content catalog. The renderer-side catalog model
requires strictly increasing revisions.

Grammar loading deliberately does not accept a URI or `IFileService`.
Extension/resource ownership must resolve and validate a contribution before
supplying its loader. This keeps Worker tokenization independent of platform
I/O and prevents `base` from learning editor concepts.

`TextMateGrammarService` is the renderer-side contribution service. It owns
registrations, cancels superseded materialization, publishes only the newest
complete catalog, and preserves the last good revision when a loader fails.
`whenReady()` lets a composition root gate work on the latest requested
revision; `onDidFailCatalog` reports a failed revision without corrupting the
catalog already used by a Worker.

`BrowserTextMateGrammarService` currently contributes the real VS Code JSON
and JSONC grammars through Vite raw resources. Those files stay under the
TextMate browser boundary and are transferred as catalog content; neither
common code nor the dedicated Worker reads product or workspace files.
Alpha now prefers App Server tree-sitter tokens for JSON/JSONC, so these
grammars remain the failure fallback and compatibility reference rather than
the normal token path.

The product Alpha session currently owns one
`BrowserTextMateAnalysisWorkerSupport`, so its catalog and Analysis Worker
share the pane session lifetime. This is intentional while only static bundled
grammars exist. Promoting the grammar service into shared Workbench
instantiation before an extension contribution host becomes a real cross-pane
consumer would leak an editor-specific dependency into products that do not
select Alpha. When that consumer exists, the shared product composition root
may own one `ITextMateGrammarService`; individual model coordinators should
still own their dedicated Analysis Workers.

## Tokenization path

1. `TextMateTokenizationService` captures the current grammar snapshot.
2. A `vscode-textmate.Registry` loads the requested root grammar and its
   injections against that exact snapshot.
3. Lines tokenize in order with immutable `StateStack` input/output state.
4. The scope resolver maps named scopes to Alpha token types.
5. Relative line tokens aggregate into an immutable `LanguageTokenResult`.
6. `createTextMateAnalysisProvider` publishes the result through Alpha's
   request-version and application gates.

The default resolver maps conventional comment, string, regexp, number,
operator, keyword, function, type, parameter, variable, tag, property,
constant, punctuation, and invalid scopes. A product theme or language adapter
may inject a stricter resolver without changing the tokenizer.

TextMate uses `tokenPriority: 100`; Alpha's deterministic lexical fallback uses
the default priority `0`. The TextMate provider intentionally declares `*` and
returns `undefined` when the current catalog has no root grammar for a language.
Alpha tries token providers in descending priority, so unsupported languages,
provider omissions, and isolated failures continue to the lexical fallback.
Equal priorities preserve registration order.

## Worker catalog path

`TextMateGrammarCatalogWireClient` sends complete validated catalog revisions
over the same structural port used by Alpha's Analysis and provider-module
protocols. `TextMateGrammarCatalogWireServer` atomically builds a new registry
before swapping the Worker-side store. Stale or malformed revisions poison the
catalog client and invalidate that Worker so Alpha's coordinator can rebuild it
from the catalog source's current revision.

`TextMateAnalysisModuleWorkerClient` serializes catalog updates and gates every
Analysis request on the latest scheduled revision. The dedicated browser Worker
activates both `textmate.grammars` and `alpha.lexical`; it owns the catalog
store, TextMate service, Oniguruma runtime, provider registries, and all three
wire servers. A replacement Worker accepts the source's current revision even
when its revision is greater than one.

## Incremental state

The service owns one latest document analysis per loaded language. It compares
old and new line arrays, reuses the unchanged prefix, and rescans from the first
changed line until an unchanged suffix line has the same TextMate input
`StateStack`. The remaining suffix is then reused without tokenization.
`synchronizeDocument` eagerly applies the same path when Alpha's Worker mirror
publishes a model transaction.

A grammar registry revision creates a new runtime generation. Requests already
using an old generation finish against their captured snapshot; its TextMate
registry is disposed after the last request releases it. Same-model-version
requests therefore cannot reuse state produced by an older grammar revision.

## Failure semantics

- invalid scope/language identities and duplicate roots fail before mutation;
- a loader returning a different root scope rejects that request;
- cancellation is checked before grammar load, after asynchronous load, and
  between every line;
- a TextMate `stoppedEarly` result rejects the provider instead of publishing a
  structurally incomplete state stack;
- resolver output is validated before it enters Alpha;
- service disposal does not dispose the caller's grammar registry or
  Oniguruma promise.

Alpha's Analysis host isolates provider failure and keeps its versioned store
unchanged or publishes the host's empty fallback according to the existing
lane contract.

## Current limitations

- VS Code JSON and JSONC grammars are bundled; TypeScript, JavaScript, embedded
  grammars, injections, and external extension contributions are not yet
  included;
- theme selector matching, embedded-language identity, token-type overrides,
  balanced-bracket selectors, and semantic modifiers are not projected yet;
- the cache aggregates a complete renderer token array after line reuse;
- `BrowserTextMateAnalysisWorkerSupport` owns the built-in catalog and matching
  Worker factory; `createBrowserAlphaEditorSession` selects it for product
  Alpha panes;
- Alpha sessions schedule a new analysis request when the catalog changes;
  other consumers must still make that scheduling decision explicitly.

Tests under `test/common` load the real `vscode-oniguruma` WASM binary and a
real TextMate grammar. They cover registry revisions, injections, cross-line
strings, scope mapping, one-line suffix reuse, multiline convergence,
same-version grammar replacement, cancellation, ownership, malformed loaders,
provider priority/fallback, catalog materialization, atomic replacement,
structured-clone wire updates, stale-client poisoning, dynamic Worker catalog
changes, and end-to-end Alpha Analysis requests. A standalone Vite build checks
the complete browser Worker and emitted WASM asset. The real bundled JSON
grammar is also tokenized through the common service in the Node test realm.
