import { registerEditorContribution } from "../../../browser/editorContribution.js";
import { DiagnosticNavigationController } from "./gotoError.js";
import { TextEditorCapability } from "../../textEditorCapabilities.js";
import { createStanzaLanguageDiagnosticSource } from "./languageDiagnosticPresentation.js";

registerEditorContribution({ id: "editor.contrib.gotoError", configure: context => {
	context.addDecorationSource(createStanzaLanguageDiagnosticSource(context.getCapability(TextEditorCapability.diagnosticDecorations)));
}, install: context => {
	if (context.kind !== "text") return;
	context.own(new DiagnosticNavigationController(context.textInput.element, context.viewport, context.selections, context.getCapability(TextEditorCapability.diagnosticDecorations)));
} });
