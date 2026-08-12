import { registerEditorContribution } from "../../../browser/editorContribution.js";
import { MessageController } from "./messageController.js";

registerEditorContribution({
  id: "editor.contrib.message",
  install: context => {
    if (context.kind !== "text") return;
    context.own(new MessageController(context.viewport));
  },
});
