import { h, reset } from "../../../../base/browser/dom.js";
import { appendIcon } from "../../../../base/browser/ui/icon/icon.js";
import { type ResolvedDecoration } from "../decorations/decorationPresentation.js";
import { type EditorOverlayContext } from "../../view/renderingContext.js";
import { type LinesDecorationLaneLayout } from "./linesDecorationsPart.js";

/** Projects line-side decoration classes into the currently rendered rows. */
export function projectStanzaLinesDecorations(
	context: EditorOverlayContext,
	decorations: readonly ResolvedDecoration[],
	lanes: ReadonlyMap<string, LinesDecorationLaneLayout>,
	scrollLeft: number,
	rows: ReadonlyMap<number, HTMLElement>,
): void {
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

	for (const row of rows.values()) reset(row);
	for (const [visualLineIndex, row] of rows) {
		const visualLine = context.visualLineProjection.lineAt(visualLineIndex);
		if (!visualLine || !visualLine.firstForLogicalLine) continue;
		for (const decoration of decorationsByLogicalLine.get(visualLine.logicalLineIndex) ?? []) {
			const presentation = decoration.linesDecoration!;
			const lane = lanes.get(presentation.owner);
			if (!lane) throw new RangeError(`Lines decoration owner '${presentation.owner}' has no layout lane`);
			const classes = [
				presentation.className,
				visualLine.logicalLineIndex === decoration.range.start.lineIndex
					? presentation.firstLineClassName
					: undefined,
			].filter((className): className is string => className !== undefined);
			const element = presentation.icon ? h(context.ownerDocument, "button") : h(context.ownerDocument, "div");
			if (presentation.icon) {
				const button = element as HTMLButtonElement;
				button.type = "button";
				button.setAttribute("aria-label", presentation.ariaLabel!);
				if (presentation.expanded === undefined) button.removeAttribute("aria-expanded");
				else button.setAttribute("aria-expanded", String(presentation.expanded));
			} else {
				element.setAttribute("aria-hidden", "true");
			}
			element.className = "stanza-editor-line-decoration";
			for (const className of classes.flatMap(value => value.trim().split(/\s+/u))) {
				element.classList.add(className);
			}
			element.dataset.decorationId = String(decoration.id);
			element.dataset.decorationOwner = presentation.owner;
			element.dataset.logicalLineIndex = String(visualLine.logicalLineIndex);
			element.style.setProperty("--stanza-editor-line-decoration-offset", `${scrollLeft + lane.left}px`);
			element.style.setProperty("--stanza-editor-line-decoration-width", `${lane.width}px`);
			const tooltip = presentation.tooltip ?? decoration.hoverText;
			if (tooltip !== undefined) element.title = tooltip;
			if (presentation.icon) {
				appendIcon(presentation.icon, element);
				element.dataset.iconId = presentation.icon.id;
			}
			row.append(element);
		}
	}
}
