import { registerTextEditorCapabilityContribution } from "../../../browser/editorExtensions.js";
import { EditorStickyScrollController } from "./stickyScrollController.js";
import { TextEditorCapability } from "../../textEditorCapabilities.js";

registerTextEditorCapabilityContribution({ id: "editor.contrib.stickyScroll", install: context => {
	if (context.kind !== "text" || context.options.stickyScroll === false || context.model.largeFile.tooLargeForTokenization) return;
	context.register(new EditorStickyScrollController(context.viewport, context.getCapability(TextEditorCapability.folding)));
} });
