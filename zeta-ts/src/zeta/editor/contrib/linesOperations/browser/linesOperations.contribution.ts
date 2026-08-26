import { registerEditorContribution } from "../../../browser/editorExtensions.js";
import { LineJoinController } from "./lineJoinController.js";
import { LineOperationsController } from "./lineOperationsController.js";

registerEditorContribution({ id: "editor.contrib.linesOperations", install: context => {
	if (context.kind !== "text") return;
	context.own(new LineOperationsController(context.view.element, context.viewport, context.selections, { indentation: context.options.indentation }));
	context.own(new LineJoinController(context.view.element, context.viewport, context.selections));
} });
