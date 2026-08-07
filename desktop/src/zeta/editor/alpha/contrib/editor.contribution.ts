import { registerEditorPane } from "../../../workbench/browser/parts/editor/editorRegistry.js";
import { DiffEditorPane } from "../browser/diffEditorPane.js";
import { RustDiffComputationService } from "../browser/services/rustDiffComputationService.js";
import { EditorPane } from "../browser/editorPane.js";
import { createBrowserEditorPart } from "../browser/browserEditorPart.js";
import { getBrowserTextModelService } from "../browser/services/browserTextModelService.js";
import { getBrowserTextResourceStore } from "../browser/services/browserTextResourceStore.js";
import { ALPHA_DIFF_EDITOR_ID, matchAlphaDiffEditor } from "../browser/diffEditorInput.js";
import { ALPHA_EDITOR_ID, matchAlphaEditor } from "../browser/editorInput.js";

registerEditorPane({
  id: ALPHA_EDITOR_ID,
  name: "Alpha Editor",
  canOpen: matchAlphaEditor,
  create: options => {
    if (!options.textFileService) throw new Error("Alpha Editor requires the Workbench text file service");
    const resourceStore = getBrowserTextResourceStore(options.textFileService);
    return new EditorPane(resourceStore, {
      modelService: getBrowserTextModelService(resourceStore),
      createPart: createBrowserEditorPart,
      textMateService: options.textMateService,
      languageFeaturesService: options.languageFeaturesService,
      syntaxApi: options.syntaxApi,
      workingCopyService: options.workingCopyService,
      onSave: options.onSave,
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
    return new DiffEditorPane(resourceStore, {
      modelService: getBrowserTextModelService(resourceStore),
      createComputationService: () => new RustDiffComputationService(diffApi),
    });
  },
});
