import { registerEditorContribution } from "../../../browser/editorExtensions.js";
import { TextEditorCapability } from "../../textEditorCapabilities.js";
import { ClipboardController } from "./clipboardController.js";
import { UriListPasteProvider } from "./clipboardPasteProvider.js";

registerEditorContribution({
	id: "editor.contrib.clipboard",
	install: context => {
		if (context.kind !== "text") return;
		context.own(new ClipboardController(context.view.editContext, context.viewport, context.selections, {
			semanticTokens: context.getOptionalCapability(TextEditorCapability.semanticTokenSource),
			isEditingAllowed: () => !context.view.compositionController.composing,
			pasteProviders: [UriListPasteProvider],
		}));
	},
});
