import { registerEditorContribution } from "../../../browser/editorContribution.js";
import { FindController } from "./findController.js";
import { TextEditorCapability } from "../../textEditorCapabilities.js";
import { TextDecorationCollection } from "../../../common/model/decorationCollection.js";
import { DecorationPresentation, createStanzaDecorationSource } from "../../../browser/viewparts/decorations/decorationPresentation.js";

registerEditorContribution({
	id: "editor.contrib.find",
	configure: context => {
		const decorations = context.own(new TextDecorationCollection<void>(context.model));
		context.provideCapability(TextEditorCapability.searchDecorations, decorations);
		context.addDecorationSource(createStanzaDecorationSource(decorations, () => DecorationPresentation.SearchMatch));
	},
	install: context => {
		if (context.kind !== "text") return;
		context.own(new FindController(context.textInput.element, context.viewport, context.selections, context.getCapability(TextEditorCapability.searchDecorations), context.options.find));
	},
});
