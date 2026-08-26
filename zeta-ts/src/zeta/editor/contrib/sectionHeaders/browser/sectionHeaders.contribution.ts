import { registerEditorContribution } from "../../../browser/editorExtensions.js";
import { SectionHeadersController } from "./sectionHeadersController.js";
import { TextEditorCapability } from "../../textEditorCapabilities.js";

registerEditorContribution({ id: "editor.contrib.sectionHeaders", install: context => {
	if (context.kind !== "text" || context.model.largeFile.tooLargeForTokenization) return;
	context.own(new SectionHeadersController(context.viewport, context.getCapability(TextEditorCapability.folding)));
} });
