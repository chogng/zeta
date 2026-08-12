import { registerEditorContribution } from "../../../browser/editorContribution.js";
import { SmartSelectController } from "./smartSelectController.js";

registerEditorContribution({
  id: "editor.contrib.smartSelect",
  install: context => {
    if (context.kind !== "text") return;
    context.own(new SmartSelectController(context.textInput.element, context.viewport, context.selections, () => context.configurations.getLanguageConfiguration(context.languageId).wordPattern));
  },
});
