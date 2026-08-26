import { registerEditorContribution } from "../../../browser/editorExtensions.js";
import { MultiCursorController } from "./multiCursorController.js";
import { OccurrenceSelectionController } from "./occurrenceSelectionController.js";

registerEditorContribution({ id: "editor.contrib.multicursor", install: context => {
	if (context.kind !== "text") return;
	context.own(new MultiCursorController(context.view.element, context.viewport, context.selections));
	context.own(new OccurrenceSelectionController(context.view.element, context.viewport, context.selections, {
		wordPattern: () => context.configurations.getLanguageConfiguration(context.languageId).wordPattern,
	}));
} });
