import { registerEditorContribution } from "../../../browser/editorContribution.js";
import { AnchorSelectController } from "./anchorSelectController.js";

registerEditorContribution({
  id: "editor.contrib.anchorSelect",
  install: context => {
    if (context.kind !== "text") return;
    context.own(new AnchorSelectController(context.textInput.element, context.viewport, context.selections, () => context.configurations.getLanguageConfiguration(context.languageId).wordPattern));
  },
});
