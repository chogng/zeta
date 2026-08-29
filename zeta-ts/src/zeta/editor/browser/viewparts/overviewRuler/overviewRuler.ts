import { type DecorationPresentation } from '../decorations/decorations.js';

export interface DiagnosticOverviewMarker {
	readonly startLineIndex: number;
	readonly endLineIndexExclusive: number;
	readonly presentation: DecorationPresentation;
	readonly hoverText: string | undefined;
}

export interface DiffOverviewMarker {
	readonly startLineIndex: number;
	readonly endLineIndexExclusive: number;
	readonly presentation: DecorationPresentation.DiffAdded | DecorationPresentation.DiffModified | DecorationPresentation.DiffDeleted;
	readonly hoverText: string | undefined;
}
