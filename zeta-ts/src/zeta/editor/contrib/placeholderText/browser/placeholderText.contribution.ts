import { registerEditorContribution } from "../../../browser/editorContribution.js";
import { PlaceholderTextController } from "./placeholderTextController.js";

registerEditorContribution({
  id: "editor.contrib.placeholderText",
  install: context => {
    if (context.kind !== "text" || !context.options.placeholder) return;
    context.own(new PlaceholderTextController(context.viewport, context.options.placeholder));
  },
});
