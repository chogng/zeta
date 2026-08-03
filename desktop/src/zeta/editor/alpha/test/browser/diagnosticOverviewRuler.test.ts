import assert from "node:assert/strict";
import test from "node:test";
import { AlphaDecorationPresentation, type AlphaResolvedDecoration } from "../../browser/decorationPresentation.js";
import { createAlphaDiagnosticOverviewMarkers } from "../../language/browser/diagnosticOverviewRuler.js";
import { TextPosition, TextRange } from "../../common/text.js";

function diagnostic(startLineIndex: number, endLineIndex: number, presentation: AlphaDecorationPresentation, hoverText: string): AlphaResolvedDecoration {
  return Object.freeze({
    id: startLineIndex + 1 as AlphaResolvedDecoration["id"],
    range: TextRange.from(TextPosition.at(startLineIndex, 0), TextPosition.at(endLineIndex, 1)),
    presentation,
    hoverText,
  });
}

test("Diagnostic overview condenses sorted logical lines by highest severity and hover text", () => {
  const markers = createAlphaDiagnosticOverviewMarkers([
    diagnostic(2, 2, AlphaDecorationPresentation.WarningUnderline, "warning"),
    diagnostic(0, 1, AlphaDecorationPresentation.InformationUnderline, "information"),
    diagnostic(1, 1, AlphaDecorationPresentation.ErrorUnderline, "error"),
    diagnostic(3, 3, AlphaDecorationPresentation.WarningUnderline, "warning"),
  ], 5);
  assert.deepEqual(markers, [
    {
      startLineIndex: 0,
      endLineIndexExclusive: 1,
      presentation: AlphaDecorationPresentation.InformationUnderline,
      hoverText: "information",
    },
    {
      startLineIndex: 1,
      endLineIndexExclusive: 2,
      presentation: AlphaDecorationPresentation.ErrorUnderline,
      hoverText: "information\nerror",
    },
    {
      startLineIndex: 2,
      endLineIndexExclusive: 4,
      presentation: AlphaDecorationPresentation.WarningUnderline,
      hoverText: "warning",
    },
  ]);
});

test("Diagnostic overview validates its document extent", () => {
  assert.throws(() => createAlphaDiagnosticOverviewMarkers([], 0), /positive line count/);
});
