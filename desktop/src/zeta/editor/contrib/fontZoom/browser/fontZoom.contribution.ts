import { registerEditorContribution } from "../../../browser/editorContribution.js";
import { FontZoomController } from "./fontZoomController.js";

registerEditorContribution({
  id: "editor.contrib.fontZoom",
  install: context => {
    if (context.kind !== "text") return;
    context.own(new FontZoomController(context.textInput.element, context.viewport, { baseLineHeight: 20, initialScale: context.options.fontZoom?.initialScale }));
  },
});
