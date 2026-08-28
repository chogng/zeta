import { type ResolvedDecoration } from "../decorations/decorationPresentation.js";
import { type EditorVisualLineProjection } from "../../../common/viewModel/modelLineProjection.js";
import { type EditorViewportLayout } from "../../../common/viewLayout/viewLayout.js";
import { type EditorOverlayContext } from "../../view/renderingContext.js";

export interface BlockDecorationGeometry {
	readonly top: number;
	readonly bottom: number;
	readonly left: number;
	readonly width: number;
	readonly padding: readonly [number, number, number, number];
}

/** Resolves one block decoration into content-coordinate geometry. */
export function resolveStanzaBlockDecorationGeometry(
	context: EditorOverlayContext,
	layout: EditorViewportLayout,
	decoration: ResolvedDecoration,
): BlockDecorationGeometry | undefined {
	const presentation = decoration.blockDecoration;
	if (!presentation) return undefined;
	const projection = context.visualLineProjection;
	const startVisualLineIndex = firstVisualLineIndex(projection, decoration.range.start.lineIndex);
	if (startVisualLineIndex === undefined) return undefined;

	const lineTop = createLineTopReader(layout);
	let top: number;
	let bottom: number;
	if (presentation.isAfterEnd) {
		const endVisualLineIndex = lastVisualLineIndex(projection, decoration.range.end.lineIndex);
		if (endVisualLineIndex === undefined) return undefined;
		top = lineTop(endVisualLineIndex + 1);
		bottom = top;
	} else {
		const endLogicalLineIndex = lastLogicalLineIndex(decoration);
		const endVisualLineIndex = lastVisualLineIndex(projection, endLogicalLineIndex);
		if (endVisualLineIndex === undefined) return undefined;
		top = lineTop(startVisualLineIndex);
		bottom = decoration.range.empty && !presentation.doesNotCollapse
			? top
			: lineTop(endVisualLineIndex + 1);
	}

	const padding = presentation.padding ?? [0, 0, 0, 0];
	const contentLeft = context.textLeft;
	return Object.freeze({
		top,
		bottom,
		left: contentLeft - padding[3],
		width: Math.max(0, layout.contentSize.width - contentLeft) + padding[1] + padding[3],
		padding,
	});
}

function lastLogicalLineIndex(decoration: ResolvedDecoration): number {
	const { start, end } = decoration.range;
	return end.columnIndex === 0 && end.lineIndex > start.lineIndex
		? end.lineIndex - 1
		: end.lineIndex;
}

function firstVisualLineIndex(projection: EditorVisualLineProjection, logicalLineIndex: number): number | undefined {
	if (logicalLineIndex < 0 || logicalLineIndex >= projection.logicalLineCount) return undefined;
	const visualLineIndex = projection.firstVisualLineIndex(logicalLineIndex);
	return projection.lineAt(visualLineIndex)?.logicalLineIndex === logicalLineIndex
		? visualLineIndex
		: undefined;
}

function lastVisualLineIndex(projection: EditorVisualLineProjection, logicalLineIndex: number): number | undefined {
	const first = firstVisualLineIndex(projection, logicalLineIndex);
	if (first === undefined) return undefined;
	let last = first;
	for (let visualLineIndex = first + 1; visualLineIndex < projection.visualLineCount; visualLineIndex += 1) {
		if (projection.lineAt(visualLineIndex)?.logicalLineIndex !== logicalLineIndex) break;
		last = visualLineIndex;
	}
	return last;
}

function createLineTopReader(layout: EditorViewportLayout): (visualLineIndex: number) => number {
	const paddingTop = layout.renderTop - layout.renderLines.startLineIndex * layout.lineHeight;
	return visualLineIndex => paddingTop + visualLineIndex * layout.lineHeight;
}
