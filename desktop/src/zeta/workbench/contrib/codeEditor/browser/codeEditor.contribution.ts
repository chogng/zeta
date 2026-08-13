import { RustDiffComputationService } from "../../../../editor/browser/services/rustDiffComputationService.js";
import { getBrowserTextModelService } from "../../../../editor/browser/services/browserTextModelService.js";
import { registerEditorPane } from "../../../browser/parts/editor/editorRegistry.js";
import { getBrowserTextResourceStore } from "./browserTextResourceStore.js";
import { createBrowserEditorPart } from "./browserEditorPart.js";
import { CODE_EDITOR_ID, matchCodeEditor } from "./codeEditorInput.js";
import { CodeEditorPane } from "./codeEditorPane.js";
import { DIFF_EDITOR_ID, matchDiffEditor } from "./diffEditorInput.js";
import { DiffEditorPane } from "./diffEditorPane.js";

registerEditorPane({
  id: CODE_EDITOR_ID,
  name: "Code Editor",
  canOpen: matchCodeEditor,
  create: options => {
    if (!options.textFileService) throw new Error("Code Editor requires the Workbench text file service");
    const resourceStore = getBrowserTextResourceStore(options.textFileService);
    return new CodeEditorPane(resourceStore, {
      modelService: getBrowserTextModelService(resourceStore),
      createPart: createBrowserEditorPart,
      textMateService: options.textMateService,
      languageFeaturesService: options.languageFeaturesService,
      syntaxApi: options.syntaxApi,
      languageDiagnosticsService: options.languageDiagnosticsService,
      workingCopyService: options.workingCopyService,
      onSave: options.onSave,
      onOpenLocation: options.onOpenLocation,
      onApplyWorkspaceEdit: options.onApplyWorkspaceEdit,
      createLineGutterDecorations: options.createLineGutterDecorations,
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
