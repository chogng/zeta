import { registerEditorContribution } from "../../../browser/editorExtensions.js";
import { InsertFinalNewLineController } from "./insertFinalNewLineController.js";

registerEditorContribution({ id: "editor.contrib.insertFinalNewLine", install: context => {
	if (context.kind !== "text" || !context.options.insertFinalNewLine) return;
	const controller = context.register(new InsertFinalNewLineController(context.selections));
	context.register(context.registerBeforeSave(() => controller.prepareSave()));
} });
