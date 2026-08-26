import { registerEditorContribution } from "../../../browser/editorExtensions.js";
import { BlockCommentController } from "./blockCommentController.js";
import { LineCommentController } from "./lineCommentController.js";
import { TextEditorCapability } from "../../textEditorCapabilities.js";

registerEditorContribution({ id: "editor.contrib.comment", install: context => {
	if (context.kind !== "text") return;
	const options = { languageId: context.languageId, configurations: context.configurations, lexicalContext: context.getOptionalCapability(TextEditorCapability.languageLexicalContext) };
	context.own(new LineCommentController(context.view.element, context.viewport, context.selections, options));
	context.own(new BlockCommentController(context.view.element, context.viewport, context.selections, options));
} });
