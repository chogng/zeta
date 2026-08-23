import assert from "node:assert/strict";
import test from "node:test";
import { DecorationPresentation, type ResolvedDecoration } from "../../../../browser/viewparts/decorations/decorationPresentation.js";
import { createAsterDiagnosticOverviewMarkers } from "../../../../browser/viewparts/overviewRuler/diagnosticOverviewMarkers.js";
import { TextPosition, TextRange } from "../../../../common/core/text.js";

function diagnostic(startLineIndex: number, endLineIndex: number, presentation: DecorationPresentation, hoverText: string): ResolvedDecoration {
	return Object.freeze({
		id: startLineIndex + 1 as ResolvedDecoration["id"],
		range: TextRange.from(TextPosition.at(startLineIndex, 0), TextPosition.at(endLineIndex, 1)),
		presentation,
		hoverText,
	});
}

test("Diagnostic overview condenses sorted logical lines by highest severity and hover text", () => {
	const markers = createAsterDiagnosticOverviewMarkers([
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
	assert.throws(() => createAsterDiagnosticOverviewMarkers([], 0), /positive line count/);
});
