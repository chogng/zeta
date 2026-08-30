import assert from "node:assert/strict";
import test from "node:test";
import { DecorationPresentation, type ResolvedDecoration } from "../../../../browser/viewParts/decorations/decorations.js";
import { createStanzaDiagnosticOverviewMarkers } from "../../../../browser/viewParts/decorations/decorations.js";
import { Position } from "../../../../common/core/position.js";
import { Range } from "../../../../common/core/range.js";

function diagnostic(startLineIndex: number, endLineIndex: number, presentation: DecorationPresentation, hoverText: string): ResolvedDecoration {
	return Object.freeze({
		id: startLineIndex + 1 as ResolvedDecoration["id"],
		range: Range.fromPositions(new Position((startLineIndex) + 1, (0) + 1), new Position((endLineIndex) + 1, (1) + 1)),
		presentation,
		hoverText,
	});
}

test("Diagnostic overview condenses sorted logical lines by highest severity and hover text", () => {
	const markers = createStanzaDiagnosticOverviewMarkers([
		diagnostic(2, 2, DecorationPresentation.WarningUnderline, "warning"),
		diagnostic(0, 1, DecorationPresentation.InformationUnderline, "information"),
		diagnostic(1, 1, DecorationPresentation.ErrorUnderline, "error"),
		diagnostic(3, 3, DecorationPresentation.WarningUnderline, "warning"),
	], 5);
	assert.deepEqual(markers, [
		{
			startLineIndex: 0,
			endLineIndexExclusive: 1,
			presentation: DecorationPresentation.InformationUnderline,
			hoverText: "information",
		},
		{
			startLineIndex: 1,
			endLineIndexExclusive: 2,
			presentation: DecorationPresentation.ErrorUnderline,
			hoverText: "information\nerror",
		},
		{
			startLineIndex: 2,
			endLineIndexExclusive: 4,
			presentation: DecorationPresentation.WarningUnderline,
			hoverText: "warning",
		},
	]);
});

test("Diagnostic overview validates its document extent", () => {
	assert.throws(() => createStanzaDiagnosticOverviewMarkers([], 0), /positive line count/);
});
