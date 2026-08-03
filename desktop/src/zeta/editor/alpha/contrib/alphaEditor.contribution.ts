import { registerEditorPane } from "../../../workbench/browser/parts/editor/editorRegistry.js";
import { AlphaDiffEditorPane } from "../browser/alphaDiffEditorPane.js";
import { AlphaEditorPane } from "../browser/alphaEditorPane.js";
import { createBrowserAlphaEditorSession } from "../browser/browserAlphaEditorSession.js";
import { ALPHA_DIFF_EDITOR_ID, matchAlphaDiffEditor } from "../common/alphaDiffEditorInput.js";
import { ALPHA_EDITOR_ID, matchAlphaEditor } from "../common/alphaEditorInput.js";

registerEditorPane({
  id: ALPHA_EDITOR_ID,
  name: "Alpha Editor",
  canOpen: matchAlphaEditor,
  create: options => {
    if (!options.textFileService) throw new Error("Alpha Editor requires the Workbench text file service");
    return new AlphaEditorPane(options.textFileService, {
      createSession: createBrowserAlphaEditorSession,
    });
  },
});

registerEditorPane({
  id: ALPHA_DIFF_EDITOR_ID,
  name: "Alpha Diff Editor",
  canOpen: matchAlphaDiffEditor,
  create: options => {
    if (!options.textFileService) throw new Error("Alpha Diff Editor requires the Workbench text file service");
    return new AlphaDiffEditorPane(options.textFileService);
  },
});
