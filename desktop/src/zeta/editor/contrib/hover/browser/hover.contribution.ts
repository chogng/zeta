import { registerEditorContribution } from "../../../browser/editorContribution.js";
import { DiagnosticHoverController } from "./diagnosticHoverController.js";
import { HoverController } from "./hoverController.js";

registerEditorContribution({ id: "editor.contrib.hover", install: context => {
  if (context.kind !== "text") return;
  context.own(new DiagnosticHoverController(context.viewport));
  context.own(new HoverController(context.viewport, context.own(context.languageFeaturesService.createHoverService(context.model)), context.languageId));
} });
