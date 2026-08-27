import { registerEditorContribution } from "../../../browser/editorExtensions.js";
import { SymbolIconsController } from "./symbolIconsController.js";
import { TextEditorCapability } from "../../textEditorCapabilities.js";

registerEditorContribution({ id: "editor.contrib.symbolIcons", install: context => {
	if (context.kind !== "text" || context.model.largeFile.tooLargeForTokenization) return;
	const service = context.register(context.languageFeaturesService.createDocumentSymbolService(context.model, { resource: context.options.input.resource, fallbackProviders: context.getOptionalCapability(TextEditorCapability.documentSymbolProviders) ?? [] }));
	context.register(new SymbolIconsController(context.viewport, service, context.languageId, context.onLanguageError));
} });
