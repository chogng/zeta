import './highlightDecorations.css';
import { DocumentHighlightKind } from '../../../common/languages/documentHighlights.js';
import { DecorationPresentation, type DecorationPresentationResolution } from '../../../browser/viewparts/decorations/decorationPresentation.js';

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

export function getHighlightDecorationOptions(kind: DocumentHighlightKind | undefined): DecorationPresentationResolution {
	if (kind === DocumentHighlightKind.Write) return wordHighlightStrong;
	if (kind === DocumentHighlightKind.Text) return wordHighlightText;
	return wordHighlight;
}

export function getSelectionHighlightDecorationOptions(hasSemanticHighlights: boolean): DecorationPresentationResolution {
	return hasSemanticHighlights ? selectionHighlightWithoutOverview : selectionHighlight;
}
