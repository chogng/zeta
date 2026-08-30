import { type EditorCapability, registerTextEditorCapabilityContribution } from "../../../browser/editorExtensions.js";
import { MultiCursorController } from "./multiCursorController.js";
import { OccurrenceSelectionController } from "./occurrenceSelectionController.js";
import { SelectionHighlighter } from "./multicursor.js";
import { TextDecorationCollection } from "../../../common/model/decorationCollection.js";
import { createStanzaDecorationSource } from "../../../browser/viewParts/decorations/decorations.js";
import { resolveSelectionHighlightPresentation } from "../../wordHighlighter/browser/highlightDecorations.js";

const selectionHighlightDecorations: EditorCapability<TextDecorationCollection<boolean>> = Object.freeze({ id: "editor.capability.selectionHighlightDecorations" });

registerTextEditorCapabilityContribution({ id: "editor.contrib.multicursor", configure: context => {
	const decorations = context.register(new TextDecorationCollection<boolean>(context.model));
	context.provideCapability(selectionHighlightDecorations, decorations);
	context.addDecorationSource(createStanzaDecorationSource(decorations, decoration => resolveSelectionHighlightPresentation(decoration.metadata)));
}, install: context => {
	if (context.kind !== "text") return;
	context.register(new MultiCursorController(context.view.element, context.viewport, context.viewModel));
	context.register(new OccurrenceSelectionController(context.view.element, context.viewport, context.viewModel));
	if (!context.model.largeFile.tooLargeForTokenization) context.register(new SelectionHighlighter(context.view, context.viewModel, context.getCapability(selectionHighlightDecorations), {
		languageId: context.languageId,
		languageFeaturesService: context.languageFeaturesService,
		enabled: context.options.selectionHighlight,
		multiline: context.options.selectionHighlightMultiline,
		maxLength: context.options.selectionHighlightMaxLength,
		occurrenceHighlights: context.options.occurrencesHighlight !== "off",
	}));
} });
