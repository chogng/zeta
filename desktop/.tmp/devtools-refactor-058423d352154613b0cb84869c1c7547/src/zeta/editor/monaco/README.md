# Monaco editor subsystem

The Monaco subsystem is selected by the Code product and composed into Complete.
It does not own files, persistence, tabs, dirty state, or Markdown rendering.

- `common/monacoEditorInput.ts` owns editor matching and extension/content-type
  to Monaco language mapping. Add or override language selection here.
- `browser/monacoEnvironment.ts` owns worker construction. Change language
  services or worker bundling here.
- `browser/monacoEditorPane.ts` owns Monaco model, DOM, layout, visibility,
  focus, and disposal for one Workbench pane.
- `contrib/monacoEditor.contribution.ts` is the stable registration boundary.
- `test/` verifies matching and language policy without loading browser workers.

`EditorInput.initialText` is currently an in-memory bootstrap snapshot. A future
document service must own loading, saving, conflict handling, and model reuse.
