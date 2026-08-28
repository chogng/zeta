import { registerEditorContribution } from "../../../browser/editorExtensions.js";
import { GotoSymbolController } from "./gotoSymbolController.js";
import { TextEditorCapability } from "../../textEditorCapabilities.js";
import { DocumentSymbolService } from '../../documentSymbols/common/documentSymbols.js';
import { GotoSymbolService } from '../common/gotoSymbol.js';

registerEditorContribution({ id: "editor.contrib.gotoSymbol", install: context => {
	if (context.kind !== "text") return;
	const symbols = new DocumentSymbolService(context.model, context.languageFeaturesService.documentSymbolProvider, { resource: context.options.input.resource });
	const service = context.register(new GotoSymbolService(symbols));
	context.register(new GotoSymbolController(context.view.element, context.viewport, context.selections, service, context.languageId, context.onLanguageError));
} });
