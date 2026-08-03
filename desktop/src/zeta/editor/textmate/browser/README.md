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
`BrowserTextMateAnalysisWorkerSupport` additionally accepts caller-owned
`TextMateGrammarDefinition` contributions plus an optional caller-owned
`TextMateScopeThemeSource`, and registers them into that session before any
Worker is created. Later selector revisions are mirrored into the existing
Worker. Extension
manifest discovery and resource
resolution remain a future composition-root concern: callers supply resolved
loaders rather than granting the Worker file or extension-host access.

`textMateAnalysisWorkerMain.ts` is the complete dedicated Worker composition:
it owns TextMate and lexical fallback modules, the grammar catalog and scope-theme
stores, the Analysis/module/catalog/theme wire servers, and the Oniguruma-backed tokenization
service. `createTextMateAnalysisWorkerFactory` creates the matching renderer
client and gates requests on the latest catalog and scope-theme revisions supplied
by the caller-owned sources.
