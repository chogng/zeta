import { registerTextEditorCapabilityContribution } from "../../../browser/editorExtensions.js";
import { InsertFinalNewLineController } from "./insertFinalNewLineController.js";

registerTextEditorCapabilityContribution({ id: "editor.contrib.insertFinalNewLine", install: context => {
	if (context.kind !== "text" || !context.options.insertFinalNewLine || !context.registerBeforeSave) return;
	const controller = context.register(new InsertFinalNewLineController(context.viewModel));
	context.register(context.registerBeforeSave(() => controller.prepareSave()));
} });
