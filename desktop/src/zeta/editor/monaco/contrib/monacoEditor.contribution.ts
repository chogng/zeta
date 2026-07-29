import {
  registerEditorPane,
} from "../../../workbench/browser/parts/editor/editorRegistry.js";
import {
  MonacoEditorPane,
} from "../browser/monacoEditorPane.js";
import {
  MONACO_EDITOR_ID,
  matchMonacoEditor,
} from "../common/monacoEditorInput.js";

registerEditorPane({
  id: MONACO_EDITOR_ID,
  name: "Code Editor",
  canOpen: matchMonacoEditor,
  create: (options) => new MonacoEditorPane(
    options.configurationService,
  ),
});
