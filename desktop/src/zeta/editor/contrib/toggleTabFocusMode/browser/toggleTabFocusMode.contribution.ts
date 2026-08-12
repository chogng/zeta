import { registerEditorContribution } from "../../../browser/editorContribution.js";
import { ToggleTabFocusModeController } from "./toggleTabFocusModeController.js";

registerEditorContribution({
  id: "editor.contrib.toggleTabFocusMode",
  install: context => {
    if (context.kind !== "text") return;
    context.own(new ToggleTabFocusModeController(context.textInput.element, context.viewport));
  },
});
