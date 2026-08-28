import { registerEditorContribution } from "../../../browser/editorExtensions.js";
import { DiagnosticHoverController } from "./diagnosticHoverController.js";
import { HoverController } from "./hoverController.js";
import { HoverService } from '../common/hover.js';

registerEditorContribution({ id: "editor.contrib.hover", install: context => {
	if (context.kind !== "text") return;
	context.register(new DiagnosticHoverController(context.viewport));
	context.register(new HoverController(context.viewport, context.register(new HoverService(context.model, context.languageFeaturesService.hoverProvider, context.options.input.resource)), context.languageId));
} });
