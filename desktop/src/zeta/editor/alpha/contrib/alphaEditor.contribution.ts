import { registerEditorPane } from "../../../workbench/browser/parts/editor/editorRegistry.js";
import { AlphaDiffEditorPane } from "../browser/alphaDiffEditorPane.js";
import { RustDiffComputationService } from "../browser/services/rustDiffComputationService.js";
import { AlphaEditorPane } from "../browser/alphaEditorPane.js";
import { createBrowserAlphaEditorSession } from "../browser/browserAlphaEditorSession.js";
import { getBrowserTextModelService } from "../browser/services/browserTextModelService.js";
import { getBrowserTextResourceStore } from "../browser/services/browserTextResourceStore.js";
import { ALPHA_DIFF_EDITOR_ID, matchAlphaDiffEditor } from "../browser/alphaDiffEditorInput.js";
import { ALPHA_EDITOR_ID, matchAlphaEditor } from "../browser/alphaEditorInput.js";

registerEditorPane({
  id: ALPHA_EDITOR_ID,
  name: "Alpha Editor",
  canOpen: matchAlphaEditor,
  create: options => {
    if (!options.textFileService) throw new Error("Alpha Editor requires the Workbench text file service");
    const resourceStore = getBrowserTextResourceStore(options.textFileService);
    return new AlphaEditorPane(resourceStore, {
      modelService: getBrowserTextModelService(resourceStore),
      createSession: createBrowserAlphaEditorSession,
      textMateService: options.textMateService,
      languageFeaturesService: options.languageFeaturesService,
    });
  },
});

registerEditorPane({
  id: ALPHA_DIFF_EDITOR_ID,
  name: "Alpha Diff Editor",
  canOpen: matchAlphaDiffEditor,
  create: options => {
    if (!options.textFileService) throw new Error("Alpha Diff Editor requires the Workbench text file service");
    const diffApi = options.diffApi;
    if (!diffApi) throw new Error("Alpha Diff Editor requires the Rust diff API");
    const resourceStore = getBrowserTextResourceStore(options.textFileService);
    return new AlphaDiffEditorPane(resourceStore, {
      modelService: getBrowserTextModelService(resourceStore),
      createComputationService: () => new RustDiffComputationService(diffApi),
    });
  },
});
