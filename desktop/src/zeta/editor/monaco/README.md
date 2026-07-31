# Monaco editor subsystem

The Monaco subsystem is selected by the Code product and composed into Complete.
It does not own files, persistence, tabs, dirty state, or Markdown rendering.
It is now a transition renderer and must not define new canonical text-model,
transaction, history, selection, or language contracts. Those move upward from
the repository-owned
[`Alpha Editor`](../alpha/README.md) as each native layer becomes usable.
The canonical staged ownership and removal criteria are documented in
[`docs/editor-architecture.md`](../../../../../docs/editor-architecture.md).

- `common/monacoEditorInput.ts` owns editor matching and extension/content-type
  to Monaco language mapping. Add or override language selection here.
- `common/config/editorConfiguration.ts` owns the typed `editor.font*`
  configuration keys and their projection into Monaco construction options.
  Monaco itself remains the canonical owner of FontInfo normalization, zoom,
  pixel ratio, and DOM glyph measurement.
- `browser/monacoEnvironment.ts` owns worker construction. Change language
  services or worker bundling here.
- `browser/monacoModelService.ts` owns the realm-scoped URI-to-model pool and
  reference-counted model disposal. A later `initialText` snapshot cannot
  overwrite a model that is already in use. This is current migration debt:
  Monaco still owns the live model until the Zeta input and command layers can
  preserve native transaction and undo semantics end to end.
- `browser/monacoSyntaxTokenService.ts` owns the Rust-only Monaco semantic-token
  provider adapter, per-model serialized App Server synchronization, restart
  reopen, revision freshness checks, and compact token projection. Its data is
  tree-sitter syntax categorization, not compiler or LSP semantic facts. The
  cross-process ownership and evolution contract is canonical in
  [`docs/syntax-analysis.md`](../../../../../docs/syntax-analysis.md).
- `browser/monacoEditorPane.ts` owns Monaco DOM, layout, visibility, focus,
  and one reference to the shared model.
- `browser/monacoChatInputEditor.ts` owns the ephemeral plaintext model,
  content-driven height, submit gesture, and Monaco DOM used inside the Chat
  composer. Chat continues to own drafts, submission, and toolbar semantics.
- `contrib/monacoEditor.contribution.ts` is the stable registration boundary.
  Code and Complete register Monaco as their Chat input editor here; products
  that do not select this contribution retain Chat's textarea fallback.
- `test/` verifies matching and language policy without loading browser workers.

All `monaco-editor` imports, including `?worker` entry points, currently resolve
to the npm transition dependency. Do not import package internals or add new
Monaco-specific product contracts.

`ITextFileService` now owns file-versus-bootstrap content resolution for Monaco,
Alpha, and ProseMirror. `EditorInput.initialText` remains only an in-memory
bootstrap snapshot. Monaco still owns its transition model pool; dirty state,
saving, revert, encoding, and conflict handling are not implemented.
