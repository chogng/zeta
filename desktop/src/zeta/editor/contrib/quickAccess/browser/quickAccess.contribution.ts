import { registerEditorContribution } from "../../../browser/editorContribution.js";
import { GotoLineController } from "./quickAccessController.js";

registerEditorContribution({
  id: "editor.contrib.quickAccess",
  install: context => {
    if (context.kind !== "text") return;
    context.own(new GotoLineController(context.textInput.element, context.viewport, context.selections));
  },
});
