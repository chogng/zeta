import { registerEditorContribution } from "../../../browser/editorContribution.js";
import { FindController } from "./findController.js";

registerEditorContribution({
  id: "editor.contrib.find",
  install: context => {
    if (context.kind !== "text") return;
    context.own(new FindController(context.textInput.element, context.viewport, context.selections, context.searchDecorations));
  },
});
