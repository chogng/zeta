import { registerTextEditorCapabilityContribution } from "../../../browser/editorExtensions.js";
import { DiagnosticNavigationController } from "./gotoError.js";
import { TextEditorCapability } from "../../textEditorCapabilities.js";

registerTextEditorCapabilityContribution({ id: "editor.contrib.gotoError", install: context => {
	if (context.kind !== "text") return;
	context.register(new DiagnosticNavigationController(context.view.element, context.viewport, context.selections, context.getCapability(TextEditorCapability.diagnosticDecorations)));
} });
