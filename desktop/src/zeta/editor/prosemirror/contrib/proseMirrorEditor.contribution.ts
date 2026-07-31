import { registerEditorPane } from "../../../workbench/browser/parts/editor/editorRegistry.js";
import { ProseMirrorEditorPane } from "../browser/proseMirrorEditorPane.js";
import { matchProseMirrorEditor, PROSEMIRROR_EDITOR_ID } from "../common/proseMirrorEditorInput.js";

registerEditorPane({
  id: PROSEMIRROR_EDITOR_ID,
  name: "Academic Editor",
  canOpen: matchProseMirrorEditor,
  create: options => {
    if (!options.textFileService) throw new Error("Academic Editor requires the Workbench text file service");
    return new ProseMirrorEditorPane(options.textFileService);
  },
});
