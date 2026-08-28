import { registerEditorContribution } from "../../../browser/editorExtensions.js";
import { LanguageTokenLineIndex } from "../../../common/tokens/languageTokenLineIndex.js";
import { TextEditorCapability } from "../../textEditorCapabilities.js";
import { SemanticTokensController } from "./semanticTokensController.js";

registerEditorContribution({
	id: "editor.contrib.semanticTokens",
	configure: context => {
		const semanticTokens = context.register(context.languageFeaturesService.createSemanticTokensService(context.model, context.options.input.resource));
		const overlay = context.register(new LanguageTokenLineIndex(semanticTokens.tokens));
		context.provideCapability(TextEditorCapability.semanticTokens, semanticTokens);
		context.provideCapability(TextEditorCapability.semanticTokenOverlay, overlay);
	},
	install: context => {
		if (context.kind !== "text" || context.model.largeFile.tooLargeForTokenization || context.model.largeFile.tooLargeForSynchronization) return;
		context.register(new SemanticTokensController(context.getCapability(TextEditorCapability.semanticTokens), context.languageId, context.options.onDidChangeLanguageSupport, context.onLanguageError));
	},
});
