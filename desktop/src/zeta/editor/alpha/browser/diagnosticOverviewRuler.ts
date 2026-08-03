import { AlphaDecorationPresentation, type AlphaResolvedDecoration } from "./decorationPresentation.js";

export interface AlphaDiagnosticOverviewMarker {
  readonly startLineIndex: number;
  readonly endLineIndexExclusive: number;
  readonly presentation: AlphaDecorationPresentation;
  readonly hoverText: string | undefined;
}

const PRESENTATION_PRIORITY = new Map<AlphaDecorationPresentation, number>([
  [AlphaDecorationPresentation.ErrorUnderline, 4],
  [AlphaDecorationPresentation.WarningUnderline, 3],
  [AlphaDecorationPresentation.InformationUnderline, 2],
  [AlphaDecorationPresentation.HintUnderline, 1],
]);

/** Condenses diagnostic decoration spans into one highest-severity marker per logical line. */
export function createAlphaDiagnosticOverviewMarkers(decorations: readonly AlphaResolvedDecoration[], lineCount: number): readonly AlphaDiagnosticOverviewMarker[] {
  if (!Number.isSafeInteger(lineCount) || lineCount < 1) throw new RangeError("Diagnostic overview requires a positive line count");
  const byLine = new Map<number, AlphaResolvedDecoration[]>();
  for (const decoration of decorations) {
    if (!PRESENTATION_PRIORITY.has(decoration.presentation)) continue;
    const startLineIndex = decoration.range.start.lineIndex;
    const endLineIndex = decoration.range.end.columnIndex === 0 && decoration.range.end.lineIndex > startLineIndex
      ? decoration.range.end.lineIndex - 1
      : decoration.range.end.lineIndex;
    for (let lineIndex = startLineIndex; lineIndex <= endLineIndex; lineIndex += 1) {
      const line = byLine.get(lineIndex) ?? [];
      line.push(decoration);
      byLine.set(lineIndex, line);
    }
  }
  const markers: AlphaDiagnosticOverviewMarker[] = [];
  for (const lineIndex of [...byLine.keys()].sort((left, right) => left - right)) {
    const lineDecorations = byLine.get(lineIndex)!;
    const highest = lineDecorations.reduce((current, candidate) =>
      (PRESENTATION_PRIORITY.get(candidate.presentation) ?? 0) > (PRESENTATION_PRIORITY.get(current.presentation) ?? 0)
        ? candidate
        : current);
    const hoverText = uniqueHoverText(lineDecorations);
    const previous = markers.at(-1);
    if (previous && previous.endLineIndexExclusive === lineIndex && previous.presentation === highest.presentation && previous.hoverText === hoverText) {
      markers[markers.length - 1] = Object.freeze({
        ...previous,
        endLineIndexExclusive: lineIndex + 1,
      });
      continue;
    }
    markers.push(Object.freeze({
      startLineIndex: lineIndex,
      endLineIndexExclusive: lineIndex + 1,
      presentation: highest.presentation,
      hoverText,
    }));
  }
  return Object.freeze(markers);
}

function uniqueHoverText(decorations: readonly AlphaResolvedDecoration[]): string | undefined {
  const values = [...new Set(decorations.flatMap(decoration => decoration.hoverText === undefined ? [] : [decoration.hoverText]))];
  return values.length > 0 ? values.join("\n") : undefined;
}
