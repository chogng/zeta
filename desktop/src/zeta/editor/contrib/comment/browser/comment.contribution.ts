import { registerEditorContribution } from "../../../browser/editorContribution.js";
import { BlockCommentController } from "./blockCommentController.js";
import { LineCommentController } from "./lineCommentController.js";

registerEditorContribution({ id: "editor.contrib.comment", install: context => {
  if (context.kind !== "text") return;
  const options = { languageId: context.languageId, configurations: context.configurations };
  context.own(new LineCommentController(context.textInput.element, context.viewport, context.selections, options));
  context.own(new BlockCommentController(context.textInput.element, context.viewport, context.selections, options));
} });
