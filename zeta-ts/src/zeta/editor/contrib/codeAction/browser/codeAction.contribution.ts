import { registerEditorContribution } from "../../../browser/editorExtensions.js";
import { CodeActionController } from "./codeActionController.js";
import { TextEditorCapability } from "../../textEditorCapabilities.js";
import { TextDecorationCollection } from "../../../common/model/decorationCollection.js";
import { type LanguageDiagnostic } from "../../../common/languages/languageResults.js";

registerEditorContribution({ id: "editor.contrib.codeAction", install: context => {
	if (context.kind !== "text") return;
	const service = context.register(context.languageFeaturesService.createCodeActionService(context.model, context.options.input.resource));
	const diagnostics = context.getOptionalCapability(TextEditorCapability.diagnosticDecorations) ?? context.register(new TextDecorationCollection<LanguageDiagnostic>(context.model));
	context.register(new CodeActionController(context.view.element, context.viewport, context.selections, service, diagnostics, context.languageId, context.options.input.resource, context.options.onApplyWorkspaceEdit, context.onLanguageError));
} });
