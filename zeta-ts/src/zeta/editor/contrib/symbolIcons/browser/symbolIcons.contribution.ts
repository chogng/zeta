import { registerEditorContribution } from "../../../browser/editorExtensions.js";
import { SymbolIconsController } from "./symbolIcons.js";
import { TextEditorCapability } from "../../textEditorCapabilities.js";

registerEditorContribution({
	id: "editor.contrib.symbolIcons",
	configure: context => {
		if (context.options.showSymbolIcons === false || context.model.largeFile.tooLargeForTokenization) return;
		const service = context.register(context.languageFeaturesService.createDocumentSymbolService(context.model, {
			resource: context.options.input.resource,
			fallbackProviders: context.getOptionalCapability(TextEditorCapability.documentSymbolProviders) ?? [],
		}));
		context.addDecorationSource(context.register(new SymbolIconsController(
			context.model,
			service,
			context.languageId,
			context.onLanguageError,
		)));
	},
});
