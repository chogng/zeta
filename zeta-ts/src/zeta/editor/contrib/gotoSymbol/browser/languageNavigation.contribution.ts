import { registerEditorContribution } from "../../../browser/editorExtensions.js";
import { LanguageNavigationController } from "./languageNavigationController.js";

registerEditorContribution({ id: "editor.contrib.languageNavigation", install: context => {
	if (context.kind !== "text") return;
	const service = context.own(context.languageFeaturesService.createLanguageNavigationService(context.model, context.options.input.resource));
	context.own(new LanguageNavigationController(context.view.element, context.viewport, context.selections, service, context.options.input.resource, context.languageId, context.options.onOpenLocation, context.onLanguageError));
} });
