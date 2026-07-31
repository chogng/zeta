# TextMate browser boundary

This directory owns browser and dedicated-Worker runtime details only.

`createBrowserTextMateOnigLib` resolves Vite's `onig.wasm` asset URL, fetches
the binary once per realm, initializes `vscode-oniguruma`, and exposes only the
`IOnigLib` contract required by `vscode-textmate`.

`createBrowserTextMateTokenizationService` combines that runtime with a
caller-owned grammar snapshot source. The returned service is caller-owned.
Neither runtime helper loads extension manifests, grammar resources, themes,
or Alpha models.

`BrowserTextMateGrammarService` is the separate product-resource boundary. It
registers the bundled VS Code JSON and JSONC raw grammar assets with the common
`TextMateGrammarService`; common code sees loaders and catalog content only.
External extension manifests and resources remain future composition-root
work. `BrowserTextMateAnalysisWorkerSupport` owns this built-in catalog
service and exposes the matching dedicated Worker factory as one disposable
unit for a future Alpha editor pane.

`textMateAnalysisWorkerMain.ts` is the complete dedicated Worker composition:
it owns TextMate and lexical fallback modules, the grammar catalog store, the
Analysis/module/catalog wire servers, and the Oniguruma-backed tokenization
service. `createTextMateAnalysisWorkerFactory` creates the matching renderer
client and gates requests on the latest catalog revision supplied by the
caller-owned source.
