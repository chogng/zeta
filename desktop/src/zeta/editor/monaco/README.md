# Monaco editor subsystem

The Monaco subsystem is selected by the Code product and composed into Complete.
It does not own files, persistence, tabs, dirty state, or Markdown rendering.

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
  overwrite a model that is already in use.
- `browser/monacoEditorPane.ts` owns Monaco DOM, layout, visibility, focus,
  and one reference to the shared model.
- `contrib/monacoEditor.contribution.ts` is the stable registration boundary.
- `test/` verifies matching and language policy without loading browser workers.

`EditorInput.initialText` is currently an in-memory bootstrap snapshot. A future
document service must own loading, saving, conflict handling, and model reuse.
