import {
  registerEditorPane,
} from "../../../workbench/browser/parts/editor/editorRegistry.js";
import {
  ProseMirrorEditorPane,
} from "../browser/proseMirrorEditorPane.js";
import {
  matchProseMirrorEditor,
  PROSEMIRROR_EDITOR_ID,
} from "../common/proseMirrorEditorInput.js";

registerEditorPane({
  id: PROSEMIRROR_EDITOR_ID,
  name: "Academic Editor",
  canOpen: matchProseMirrorEditor,
  create: () => new ProseMirrorEditorPane(),
});
