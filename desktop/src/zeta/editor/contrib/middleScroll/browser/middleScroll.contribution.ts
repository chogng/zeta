import { registerEditorContribution } from "../../../browser/editorContribution.js";
import { MiddleScrollController } from "./middleScrollController.js";

registerEditorContribution({
  id: "editor.contrib.middleScroll",
  install: context => {
    if (context.kind !== "text") return;
    context.own(new MiddleScrollController(context.viewport));
  },
});
