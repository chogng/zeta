import { registerEditorContribution } from "../../../browser/editorContribution.js";
import { ContextMenuController } from "./contextMenuController.js";

registerEditorContribution({
  id: "editor.contrib.contextMenu",
  install: context => {
    if (context.kind !== "text" || !context.options.onShowContextMenu) return;
    context.own(new ContextMenuController(context.viewport, context.options.onShowContextMenu, context.options.onLanguageError));
  },
});
