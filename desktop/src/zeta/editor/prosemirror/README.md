# ProseMirror editor subsystem

The ProseMirror subsystem is selected by Academic and composed into Complete.
It owns structured rich-text editing, not Typst compilation or PDF preview.

- `common/proseMirrorEditorInput.ts` owns compatible resource and content-type
  matching.
- `common/proseMirrorEditorState.ts` owns the canonical schema, initial document
  construction, history, and base keybindings. Add paper nodes, marks, plugins,
  and agent-visible document invariants here.
- `browser/proseMirrorEditorPane.ts` owns `EditorView`, DOM, layout, focus, and
  disposal for one Workbench pane.
- `contrib/proseMirrorEditor.contribution.ts` is the stable registration
  boundary.
- `test/` verifies matching and schema/state policy without browser coupling.

The planned ProseMirror-to-Typst serializer must consume
`proseMirrorDocumentSchema`; it must not infer paper structure from rendered
DOM. Loading, saving, dirty state, diagnostics, and PDF preview remain outside
the pane.
