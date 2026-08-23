import { registerEditorContribution } from "../../../browser/editorContribution.js";
import { LanguageTokenLineIndex } from "../../../common/tokens/languageTokenLineIndex.js";
import { TextEditorCapability } from "../../textEditorCapabilities.js";
import { SemanticTokensController } from "./semanticTokensController.js";

registerEditorContribution({
  id: "editor.contrib.semanticTokens",
  configure: context => {
    const semanticTokens = context.own(context.languageFeaturesService.createSemanticTokensService(context.model, context.options.input.resource));
    const overlay = context.own(new LanguageTokenLineIndex(semanticTokens.tokens));
    context.provideCapability(TextEditorCapability.semanticTokens, semanticTokens);
    context.provideCapability(TextEditorCapability.semanticTokenOverlay, overlay);
  },
  install: context => {
    if (context.kind !== "text" || context.model.largeFile.tooLargeForTokenization || context.model.largeFile.tooLargeForSynchronization) return;
    context.own(new SemanticTokensController(context.getCapability(TextEditorCapability.semanticTokens), context.languageId, context.options.whenLanguageSupportReady ?? (() => Promise.resolve()), context.options.onDidChangeLanguageSupport, context.onLanguageError));
  },
});
