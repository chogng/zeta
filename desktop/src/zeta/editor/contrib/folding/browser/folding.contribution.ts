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
    const hiddenRanges = context.own(new EditorHiddenRangeModel(context.model, folding));
    const rustSyntaxFacts = context.getOptionalCapability(TextEditorCapability.rustSyntaxFacts);
    let syntaxFolding: RustSyntaxFoldingService | undefined;
    const update = () => folding.setProviderRanges(mergeEditorFoldingRanges(
      syntaxFolding?.ranges ?? [],
      computeEditorLanguageFoldingRanges(context.model, context.languageId, context.configurations),
      computeEditorIndentFoldingRanges(context.model),
    ));
    if (rustSyntaxFacts) syntaxFolding = context.own(new RustSyntaxFoldingService(context.model, context.languageId, rustSyntaxFacts, update, context.onLanguageError));
    update();
    context.own(context.model.onDidChange(update));
    context.provideCapability(TextEditorCapability.folding, folding);
    context.setLineProjection({ visibilitySource: hiddenRanges, gutterDecoration: new FoldingDecorationProvider(folding) });
  },
  install: context => {
    if (context.kind !== "text") return;
    context.own(new FoldingController(context.textInput.element, context.viewport, context.selections, context.getCapability(TextEditorCapability.folding)));
  },
});
