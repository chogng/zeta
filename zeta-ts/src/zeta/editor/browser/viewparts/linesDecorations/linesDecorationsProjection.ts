import { h, reset } from "../../../../base/browser/dom.js";
import { type ResolvedDecoration } from "../decorations/decorationPresentation.js";
import { type ViewportOverlayContext } from "../viewportOverlay/viewportOverlayPresentation.js";

/** Projects line-side decoration classes into the currently rendered rows. */
export function projectStanzaLinesDecorations(context: ViewportOverlayContext, decorations: readonly ResolvedDecoration[]): void {
	const decorationsByLogicalLine = new Map<number, ResolvedDecoration[]>();
	for (const decoration of decorations) {
		if (!decoration.linesDecoration) continue;
		const startLineIndex = decoration.range.start.lineIndex;
		const endLineIndex = decoration.range.end.columnIndex === 0 && decoration.range.end.lineIndex > startLineIndex
			? decoration.range.end.lineIndex - 1
			: decoration.range.end.lineIndex;
		for (let lineIndex = startLineIndex; lineIndex <= endLineIndex; lineIndex += 1) {
			const lineDecorations = decorationsByLogicalLine.get(lineIndex) ?? [];
			lineDecorations.push(decoration);
			decorationsByLogicalLine.set(lineIndex, lineDecorations);
		}
	}

	for (const line of context.renderedLines.values()) reset(line.linesDecorationElement);
	for (const [visualLineIndex, line] of context.renderedLines) {
		const visualLine = context.visualLineProjection.lineAt(visualLineIndex);
		if (!visualLine || !visualLine.firstForLogicalLine) continue;
		for (const decoration of decorationsByLogicalLine.get(visualLine.logicalLineIndex) ?? []) {
			const presentation = decoration.linesDecoration!;
			const classes = [
				presentation.className,
				visualLine.logicalLineIndex === decoration.range.start.lineIndex
					? presentation.firstLineClassName
					: undefined,
			].filter((className): className is string => className !== undefined);
			if (classes.length === 0) continue;
			const element = h(context.ownerDocument, "div");
			element.className = "stanza-editor-line-decoration";
			for (const className of classes.flatMap(value => value.trim().split(/\s+/u))) {
				element.classList.add(className);
			}
			element.dataset.decorationId = String(decoration.id);
			const tooltip = presentation.tooltip ?? decoration.hoverText;
			if (tooltip !== undefined) element.title = tooltip;
			line.linesDecorationElement.append(element);
		}
	}
}
