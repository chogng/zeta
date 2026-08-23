import { registerEditorContribution } from "../../../browser/editorContribution.js";
import { OccurrenceHighlightController } from "./wordHighlighterController.js";
import { TextEditorCapability } from "../../textEditorCapabilities.js";
import { TextDecorationCollection } from "../../../common/model/decorationCollection.js";
import { DecorationPresentation, createAsterDecorationSource } from "../../../browser/viewparts/decorations/decorationPresentation.js";

registerEditorContribution({ id: "editor.contrib.wordHighlighter", configure: context => {
	const decorations = context.own(new TextDecorationCollection<void>(context.model));
	context.provideCapability(TextEditorCapability.occurrenceDecorations, decorations);
	context.addDecorationSource(createAsterDecorationSource(decorations, () => DecorationPresentation.OccurrenceHighlight));
}, install: context => {
	if (context.kind !== "text" || context.model.largeFile.tooLargeForTokenization) return;
	context.own(new OccurrenceHighlightController(context.selections, context.getCapability(TextEditorCapability.occurrenceDecorations), {
		wordPattern: () => context.configurations.getLanguageConfiguration(context.languageId).wordPattern,
	}));
} });
