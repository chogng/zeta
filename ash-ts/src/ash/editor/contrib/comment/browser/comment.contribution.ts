import { registerTextEditorCapabilityContribution } from "../../../browser/editorExtensions.js";
import { BlockCommentController, ToggleBlockCommentCommandId } from "./blockCommentController.js";
import { LineCommentController, ToggleLineCommentCommandId } from "./lineCommentController.js";
import { TextEditorCapability } from "../../textEditorCapabilities.js";

registerTextEditorCapabilityContribution({ id: "editor.contrib.comment", commands: [
	{ id: ToggleLineCommentCommandId, canTriggerInlineEdits: true },
	{ id: ToggleBlockCommentCommandId, canTriggerInlineEdits: true },
], install: context => {
	if (context.kind !== "text") return;
	const options = { languageId: context.languageId, configurations: context.configurations, lexicalContext: context.getOptionalCapability(TextEditorCapability.languageLexicalContext) };
	context.register(new LineCommentController(context.view.element, context.viewport, context.selectionController, options, context.executeCommand));
	context.register(new BlockCommentController(context.view.element, context.viewport, context.selectionController, options, context.executeCommand));
} });
