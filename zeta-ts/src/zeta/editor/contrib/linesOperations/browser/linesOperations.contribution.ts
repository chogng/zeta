import { registerTextEditorCapabilityContribution } from "../../../browser/editorExtensions.js";
import { JoinLinesCommandId, LineJoinController } from "./lineJoinController.js";
import { EditorLineOperationCommandId, LineOperationsController } from "./lineOperationsController.js";

registerTextEditorCapabilityContribution({ id: "editor.contrib.linesOperations", commands: [
	...Object.values(EditorLineOperationCommandId).map(id => ({ id, canTriggerInlineEdits: true })),
	{ id: JoinLinesCommandId, canTriggerInlineEdits: true },
], install: context => {
	if (context.kind !== "text") return;
	context.register(new LineOperationsController(context.view.element, context.viewport, context.selections, { indentation: context.options.indentation, executeCommand: context.executeCommand }));
	context.register(new LineJoinController(context.view.element, context.viewport, context.selections, { executeCommand: context.executeCommand }));
} });
