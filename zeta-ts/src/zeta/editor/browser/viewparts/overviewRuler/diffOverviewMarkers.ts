import { DecorationPresentation, type ResolvedDecoration } from "../decorations/decorationPresentation.js";

export interface DiffOverviewMarker {
	readonly startLineIndex: number;
	readonly endLineIndexExclusive: number;
	readonly presentation: DecorationPresentation.DiffAdded | DecorationPresentation.DiffModified | DecorationPresentation.DiffDeleted;
	readonly hoverText: string | undefined;
}

const DIFF_PRESENTATIONS = new Set<DecorationPresentation>([
	DecorationPresentation.DiffAdded,
	DecorationPresentation.DiffModified,
	DecorationPresentation.DiffDeleted,
]);

/** Condenses Quick Diff decorations into stable overview-ruler line ranges. */
export function createStanzaDiffOverviewMarkers(decorations: readonly ResolvedDecoration[], lineCount: number): readonly DiffOverviewMarker[] {
	if (!Number.isSafeInteger(lineCount) || lineCount < 1) throw new RangeError("Diff overview requires a positive line count");
	const markers: DiffOverviewMarker[] = [];
	for (const decoration of decorations) {
		if (!DIFF_PRESENTATIONS.has(decoration.presentation)) continue;
		const presentation = decoration.presentation as DiffOverviewMarker["presentation"];
		const lineIndex = decoration.range.start.lineIndex;
		const previous = markers.at(-1);
		if (previous && previous.endLineIndexExclusive === lineIndex && previous.presentation === presentation && previous.hoverText === decoration.hoverText) {
			markers[markers.length - 1] = Object.freeze({ ...previous, endLineIndexExclusive: lineIndex + 1 });
			continue;
		}
		markers.push(Object.freeze({
			startLineIndex: lineIndex,
			endLineIndexExclusive: lineIndex + 1,
			presentation,
			hoverText: decoration.hoverText,
		}));
	}
	return Object.freeze(markers);
}
