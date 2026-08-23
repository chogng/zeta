import { type ResolvedDecoration, DecorationPresentation } from "../decorations/decorationPresentation.js";
import { type ViewportOverlayContext } from "../viewportOverlay/viewportOverlayPresentation.js";

const DIAGNOSTIC_PRESENTATION_PRIORITY = new Map<ResolvedDecoration["presentation"], number>([
  [DecorationPresentation.ErrorUnderline, 4],
  [DecorationPresentation.WarningUnderline, 3],
  [DecorationPresentation.InformationUnderline, 2],
  [DecorationPresentation.HintUnderline, 1],
]);

/** Projects the highest-severity diagnostic marker for each visible logical line. */
export function projectAsterDiagnosticMarginDecorations(context: ViewportOverlayContext, decorations: readonly ResolvedDecoration[]): void {
  const diagnosticsByLine = new Map<number, ResolvedDecoration[]>();
  for (const decoration of decorations) {
    if (!DIAGNOSTIC_PRESENTATION_PRIORITY.has(decoration.presentation)) continue;
    const startLineIndex = decoration.range.start.lineIndex;
    const endLineIndex = decoration.range.end.columnIndex === 0 && decoration.range.end.lineIndex > startLineIndex
      ? decoration.range.end.lineIndex - 1
      : decoration.range.end.lineIndex;
    for (let lineIndex = startLineIndex; lineIndex <= endLineIndex; lineIndex += 1) {
      const lineDiagnostics = diagnosticsByLine.get(lineIndex) ?? [];
      lineDiagnostics.push(decoration);
      diagnosticsByLine.set(lineIndex, lineDiagnostics);
    }
  }
  for (const [visualLineIndex, line] of context.renderedLines) {
    const visualLine = context.visualLineProjection.lineAt(visualLineIndex);
    const diagnostics = visualLine?.firstForLogicalLine
      ? diagnosticsByLine.get(visualLine.logicalLineIndex) ?? []
      : [];
    const marker = line.diagnosticDomNode;
    marker.setHidden(diagnostics.length === 0);
    delete marker.domNode.dataset.diagnosticHoverText;
    marker.domNode.removeAttribute("title");
    if (diagnostics.length === 0) {
      marker.setClassName("aster-editor-diagnostic-marker");
      marker.setTextContent("");
      continue;
    }
    const highest = diagnostics.reduce((current, candidate) =>
      (DIAGNOSTIC_PRESENTATION_PRIORITY.get(candidate.presentation) ?? 0) > (DIAGNOSTIC_PRESENTATION_PRIORITY.get(current.presentation) ?? 0)
        ? candidate
        : current);
    marker.setClassName(`aster-editor-diagnostic-marker ${diagnosticMarkerClass(highest.presentation)}`);
    marker.setTextContent("●");
    const hoverTexts = [...new Set(diagnostics.flatMap(diagnostic => diagnostic.hoverText === undefined ? [] : [diagnostic.hoverText]))];
    if (hoverTexts.length > 0) {
      const hoverText = hoverTexts.join("\n");
      marker.domNode.dataset.diagnosticHoverText = hoverText;
      marker.domNode.title = hoverText;
    }
  }
}

function diagnosticMarkerClass(presentation: ResolvedDecoration["presentation"]): "error" | "warning" | "information" | "hint" {
  switch (presentation) {
    case DecorationPresentation.ErrorUnderline: return "error";
    case DecorationPresentation.WarningUnderline: return "warning";
    case DecorationPresentation.InformationUnderline: return "information";
    case DecorationPresentation.HintUnderline: return "hint";
    default: throw new TypeError(`Unknown diagnostic presentation '${presentation}'`);
  }
}
