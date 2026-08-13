import { registerEditorContribution } from "../../../browser/editorContribution.js";
import { FoldingController } from "./folding.js";
import { TextEditorCapability } from "../../textEditorCapabilities.js";
import { EditorFoldingModel } from "./foldingModel.js";
import { EditorHiddenRangeModel } from "./hiddenRangeModel.js";
import { computeEditorIndentFoldingRanges } from "./indentRangeProvider.js";
import { computeEditorLanguageFoldingRanges, mergeEditorFoldingRanges } from "./syntaxRangeProvider.js";
import { FoldingDecorationProvider } from "./foldingDecorations.js";
import { RustSyntaxFoldingService } from "../../../browser/services/rustSyntaxFoldingService.js";

registerEditorContribution({
  id: "editor.contrib.folding",
  configure: context => {
    const folding = context.own(new EditorFoldingModel(context.model));
    const largeFile = context.model.largeFile.tooLargeForTokenization;
    const hiddenRanges = largeFile ? undefined : context.own(new EditorHiddenRangeModel(context.model, folding));
    const rustSyntaxFacts = largeFile ? undefined : context.getOptionalCapability(TextEditorCapability.rustSyntaxFacts);
    const languageFolding = largeFile ? undefined : context.own(context.languageFeaturesService.createFoldingRangeService(context.model, context.options.input.resource));
    let syntaxFolding: RustSyntaxFoldingService | undefined;
    let serverRanges: readonly { readonly startLineIndex: number; readonly endLineIndex: number }[] = [];
    let requestSerial = 0;
    const update = () => folding.setProviderRanges(largeFile ? [] : mergeEditorFoldingRanges(serverRanges, syntaxFolding?.ranges ?? [], computeEditorLanguageFoldingRanges(context.model, context.languageId, context.configurations), computeEditorIndentFoldingRanges(context.model)));
    const refresh = () => {
      update();
      if (!languageFolding) return;
      const serial = ++requestSerial;
      void languageFolding.provideFoldingRanges(context.languageId).then(ranges => {
        if (serial !== requestSerial) return;
        serverRanges = ranges;
        update();
      }, context.onLanguageError);
    };
    if (rustSyntaxFacts) syntaxFolding = context.own(new RustSyntaxFoldingService(context.model, context.languageId, rustSyntaxFacts, update, context.onLanguageError));
    refresh();
    if (!largeFile) context.own(context.model.onDidChange(refresh));
    context.provideCapability(TextEditorCapability.folding, folding);
    if (hiddenRanges) {
      context.setLineProjection({ visibilitySource: hiddenRanges });
      context.addLineGutterDecoration(new FoldingDecorationProvider(folding));
    }
  },
  install: context => {
    if (context.kind !== "text" || context.model.largeFile.tooLargeForTokenization) return;
    context.own(new FoldingController(context.textInput.element, context.viewport, context.selections, context.getCapability(TextEditorCapability.folding)));
  },
});
