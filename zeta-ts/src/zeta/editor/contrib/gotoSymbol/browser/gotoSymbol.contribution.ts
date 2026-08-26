import { registerEditorContribution } from "../../../browser/editorExtensions.js";
import { GotoSymbolController } from "./gotoSymbolController.js";
import { TextEditorCapability } from "../../textEditorCapabilities.js";

registerEditorContribution({ id: "editor.contrib.gotoSymbol", install: context => {
	if (context.kind !== "text") return;
	const service = context.own(context.languageFeaturesService.createGotoSymbolService(context.model, { resource: context.options.input.resource, fallbackProviders: context.getOptionalCapability(TextEditorCapability.documentSymbolProviders) ?? [] }));
	context.own(new GotoSymbolController(context.input.element, context.viewport, context.selections, service, context.languageId, context.onLanguageError));
} });
