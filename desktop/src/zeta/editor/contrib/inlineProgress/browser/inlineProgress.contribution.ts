import { registerEditorContribution } from "../../../browser/editorContribution.js";
import { InlineProgressController } from "./inlineProgressController.js";

registerEditorContribution({
  id: "editor.contrib.inlineProgress",
  install: context => {
    if (context.kind !== "text") return;
    context.own(new InlineProgressController(context.viewport));
  },
});
