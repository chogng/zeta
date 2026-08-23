import { registerEditorContribution } from "../../../browser/editorContribution.js";
import { InsertFinalNewLineController } from "./insertFinalNewLineController.js";

registerEditorContribution({ id: "editor.contrib.insertFinalNewLine", install: context => {
	if (context.kind !== "text" || !context.options.insertFinalNewLine) return;
	const controller = context.own(new InsertFinalNewLineController(context.selections));
	context.own(context.registerBeforeSave(() => controller.prepareSave()));
} });
