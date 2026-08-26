import { registerEditorContribution } from "../../../browser/editorExtensions.js";
import { LanguageHierarchyController } from "./languageHierarchyController.js";

registerEditorContribution({ id: "editor.contrib.languageHierarchy", install: context => {
	if (context.kind !== "text") return;
	const service = context.own(context.languageFeaturesService.createLanguageHierarchyService(context.model, context.options.input.resource));
	context.own(new LanguageHierarchyController(context.input.element, context.viewport, context.selections, service, context.options.input.resource, context.languageId, context.options.onOpenLocation, context.onLanguageError));
} });
