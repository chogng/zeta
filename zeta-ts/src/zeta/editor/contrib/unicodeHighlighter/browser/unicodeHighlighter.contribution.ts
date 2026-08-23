import { registerEditorContribution } from "../../../browser/editorContribution.js";
import { UnicodeHighlighterController } from "./unicodeHighlighterController.js";
import { TextEditorCapability } from "../../textEditorCapabilities.js";
import { TextDecorationCollection } from "../../../common/model/decorationCollection.js";
import { DecorationPresentation, createAsterDecorationSource } from "../../../browser/viewparts/decorations/decorationPresentation.js";
import { type UnicodeHighlight } from "../common/unicodeHighlighter.js";

registerEditorContribution({ id: "editor.contrib.unicodeHighlighter", configure: context => {
	if (context.options.showUnicodeHighlights === false) return;
	const decorations = context.own(new TextDecorationCollection<UnicodeHighlight>(context.model));
	context.provideCapability(TextEditorCapability.unicodeDecorations, decorations);
	context.addDecorationSource(createAsterDecorationSource(decorations, () => DecorationPresentation.UnicodeHighlight, decoration => `${decoration.metadata.kind} Unicode character U+${decoration.metadata.character.codePointAt(0)!.toString(16).toUpperCase()}`));
}, install: context => {
	if (context.kind !== "text" || context.options.showUnicodeHighlights === false || context.model.largeFile.tooLargeForTokenization) return;
	context.own(new UnicodeHighlighterController(context.model, context.getCapability(TextEditorCapability.unicodeDecorations)));
} });
