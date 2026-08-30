import { type DiagnosticOverviewMarker, type DiffOverviewMarker, EditorOverviewRuler } from './overviewRuler.js';

const OVERVIEW_RULER_WIDTH = 6;

export type OverviewRulerMarker = DiagnosticOverviewMarker | DiffOverviewMarker;

export interface DecorationsOverviewRulerOptions {
	readonly host: HTMLElement;
	readonly verticalScrollbarWidth: number;
	readonly getVerticalOffsetForLineIndex: (lineIndex: number) => number;
	readonly readMarkers: () => readonly OverviewRulerMarker[];
	readonly readMarkersRevision: () => number;
}

/** Projects diagnostic and diff markers into the editor's overview ruler. */
export class EditorDecorationsOverviewRuler extends EditorOverviewRuler {
	constructor(options: DecorationsOverviewRulerOptions) {
		super({
			host: options.host,
			className: 'stanza-editor-overview-ruler',
			width: OVERVIEW_RULER_WIDTH,
			verticalScrollbarWidth: options.verticalScrollbarWidth,
			getVerticalOffsetForLineIndex: options.getVerticalOffsetForLineIndex,
			readEntries: () => options.readMarkers().map(marker => ({
				startLineIndex: marker.startLineIndex,
				endLineIndexExclusive: marker.endLineIndexExclusive,
				className: marker.presentation,
				...(marker.hoverText === undefined ? {} : { hoverText: marker.hoverText }),
			})),
			readEntriesRevision: options.readMarkersRevision,
		});
	}
}
