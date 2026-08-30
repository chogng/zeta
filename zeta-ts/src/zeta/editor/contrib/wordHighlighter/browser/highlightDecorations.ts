import './highlightDecorations.css';
import { DocumentHighlightKind } from '../../../common/languages.js';
import { DecorationPresentation, type DecorationPresentationResolution } from '../../../browser/viewparts/decorations/decorations.js';

const wordHighlight = Object.freeze({
	presentation: DecorationPresentation.WordHighlight,
	overviewRuler: true,
	minimap: true,
});
const wordHighlightStrong = Object.freeze({
	presentation: DecorationPresentation.WordHighlightStrong,
	overviewRuler: true,
	minimap: true,
});
const wordHighlightText = Object.freeze({
	presentation: DecorationPresentation.WordHighlightText,
	overviewRuler: true,
	minimap: true,
});
const selectionHighlight = Object.freeze({
	presentation: DecorationPresentation.SelectionHighlight,
	overviewRuler: true,
	minimap: true,
});
const selectionHighlightWithoutOverview = Object.freeze({
	presentation: DecorationPresentation.SelectionHighlight,
	minimap: true,
});

export function resolveDocumentHighlightPresentation(kind: DocumentHighlightKind | undefined): DecorationPresentationResolution {
	if (kind === DocumentHighlightKind.Write) return wordHighlightStrong;
	if (kind === DocumentHighlightKind.Text) return wordHighlightText;
	return wordHighlight;
}

export function resolveSelectionHighlightPresentation(hasSemanticHighlights: boolean): DecorationPresentationResolution {
	return hasSemanticHighlights ? selectionHighlightWithoutOverview : selectionHighlight;
}
