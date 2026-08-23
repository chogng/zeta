import { registerEditorContribution } from "../../../browser/editorContribution.js";
import { StickyScrollController } from "./stickyScrollController.js";
import { TextEditorCapability } from "../../textEditorCapabilities.js";

registerEditorContribution({ id: "editor.contrib.stickyScroll", install: context => {
	if (context.kind !== "text" || context.options.stickyScroll === false || context.model.largeFile.tooLargeForTokenization) return;
	context.own(new StickyScrollController(context.viewport, context.getCapability(TextEditorCapability.folding)));
} });
