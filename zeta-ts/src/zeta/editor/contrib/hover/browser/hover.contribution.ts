import { registerEditorContribution } from "../../../browser/editorExtensions.js";
import { DiagnosticHoverController } from "./diagnosticHoverController.js";
import { HoverController } from "./hoverController.js";

registerEditorContribution({ id: "editor.contrib.hover", install: context => {
	if (context.kind !== "text") return;
	context.register(new DiagnosticHoverController(context.viewport));
	context.register(new HoverController(context.viewport, context.register(context.languageFeaturesService.createHoverService(context.model, context.options.input.resource)), context.languageId));
} });
