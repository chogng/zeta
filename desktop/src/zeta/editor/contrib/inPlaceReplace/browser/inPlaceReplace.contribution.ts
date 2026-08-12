import { registerEditorContribution } from "../../../browser/editorContribution.js";
import { InPlaceReplaceController } from "./inPlaceReplaceController.js";

registerEditorContribution({
  id: "editor.contrib.inPlaceReplace",
  install: context => {
    if (context.kind !== "text") return;
    context.own(new InPlaceReplaceController(context.textInput.element, context.viewport, context.selections));
  },
});
