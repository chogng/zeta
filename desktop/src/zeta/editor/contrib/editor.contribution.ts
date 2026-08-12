import { registerEditorPane } from "../../workbench/browser/parts/editor/editorRegistry.js";
import { DiffEditorPane } from "../browser/diffEditorPane.js";
import { RustDiffComputationService } from "../browser/services/rustDiffComputationService.js";
import { EditorPane } from "../browser/codeEditorPane.js";
import { createBrowserEditorPart } from "../browser/browserEditorPart.js";
import { getBrowserTextModelService } from "../browser/services/browserTextModelService.js";
import { getBrowserTextResourceStore } from "../browser/services/browserTextResourceStore.js";
import { DIFF_EDITOR_ID, matchDiffEditor } from "../browser/diffEditorInput.js";
import { CODE_EDITOR_ID, matchCodeEditor } from "../browser/codeEditorInput.js";

registerEditorPane({
  id: CODE_EDITOR_ID,
  name: "Code Editor",
  canOpen: matchCodeEditor,
  create: options => {
    if (!options.textFileService) throw new Error("Code Editor requires the Workbench text file service");
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
  id: DIFF_EDITOR_ID,
  name: "Diff Editor",
  canOpen: matchDiffEditor,
  create: options => {
    if (!options.textFileService) throw new Error("Diff Editor requires the Workbench text file service");
    const diffApi = options.diffApi;
    if (!diffApi) throw new Error("Diff Editor requires the Rust diff API");
    const resourceStore = getBrowserTextResourceStore(options.textFileService);
    return new DiffEditorPane(resourceStore, {
      modelService: getBrowserTextModelService(resourceStore),
      createComputationService: () => new RustDiffComputationService(diffApi),
    });
  },
});
