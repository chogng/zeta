import { registerEditorContribution } from "../../../browser/editorContribution.js";
import { UnusualLineTerminatorsController } from "./unusualLineTerminatorsController.js";
import { TextEditorCapability } from "../../textEditorCapabilities.js";
import { TextDecorationCollection } from "../../../common/model/decorationCollection.js";
import { DecorationPresentation, createStanzaDecorationSource } from "../../../browser/viewparts/decorations/decorationPresentation.js";

registerEditorContribution({ id: "editor.contrib.unusualLineTerminators", configure: context => {
	const decorations = context.own(new TextDecorationCollection<void>(context.model));
	context.provideCapability(TextEditorCapability.unusualLineTerminatorDecorations, decorations);
	context.addDecorationSource(createStanzaDecorationSource(decorations, () => DecorationPresentation.UnusualLineTerminator, () => "Unusual line terminator"));
}, install: context => {
	if (context.kind !== "text" || context.model.largeFile.tooLargeForTokenization) return;
	context.own(new UnusualLineTerminatorsController(context.model, context.getCapability(TextEditorCapability.unusualLineTerminatorDecorations)));
} });
