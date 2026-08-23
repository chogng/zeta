import { type EditorSelectionController } from "../../../common/cursor/editorSelectionController.js";
import { type TextRange } from "../../../common/core/text.js";
import { type TextModel } from "../../../common/model/textModel.js";
import { type EditorVisualLineProjection } from "../../../common/viewModel/modelLineProjection.js";
import { type TextMeasurer } from "../../../common/viewModel/textMeasurer.js";
import { type EditorLineRange } from "../../../common/viewLayout/editorViewportModel.js";
import { getStanzaDomTextCaretLeft, getStanzaDomTextRangeRectangles } from "./domTextGeometry.js";
import { type RenderedLine } from "../viewLines/renderedLine.js";

/** Selects whether selection projection marks the cursor's logical line as active. */
export type ActiveLineHighlight = "on" | "off";

export interface ViewportOverlayContext {
	readonly ownerDocument: Document;
	readonly model: TextModel;
	readonly visualLineProjection: EditorVisualLineProjection;
	readonly renderedLines: ReadonlyMap<number, RenderedLine>;
	readonly renderLines: EditorLineRange;
	readonly textLeft: number;
	readonly textMeasurer: TextMeasurer;
	/** Uses browser range geometry when text direction may produce non-monotonic advances. */
	readonly useDomTextGeometry: boolean;
	/** `off` matches simple input editors by omitting current-line presentation DOM. */
	readonly activeLineHighlight: ActiveLineHighlight;
}

export interface DomSelectionRectangle {
	readonly selectionIndex: number;
	readonly visualLineIndex: number;
	readonly left: number;
	readonly width: number;
}

export interface DomCaretRectangle {
	readonly selectionIndex: number;
	readonly visualLineIndex: number;
	readonly left: number;
	readonly primary: boolean;
}

export interface DomSelectionGeometry {
	readonly selectionIndexes: ReadonlySet<number>;
	readonly selections: readonly DomSelectionRectangle[];
	readonly caretIndexes: ReadonlySet<number>;
	readonly carets: readonly DomCaretRectangle[];
}

export function createStanzaDomSelectionGeometry(context: ViewportOverlayContext, selections: EditorSelectionController["selections"]): DomSelectionGeometry | undefined {
	const selectionIndexes = new Set<number>();
	const domSelections: DomSelectionRectangle[] = [];
	const caretIndexes = new Set<number>();
	const domCarets: DomCaretRectangle[] = [];
	for (let selectionIndex = 0; selectionIndex < selections.selections.length; selectionIndex += 1) {
		const selection = selections.selections[selectionIndex]!;
		if (!selection.collapsed) {
			const candidate = createStanzaDomRangeRectangles(context, selection.range);
			if (candidate) {
				selectionIndexes.add(selectionIndex);
				domSelections.push(...candidate.map(rectangle => Object.freeze({ ...rectangle, selectionIndex })));
			}
		}
		const visualLineIndex = context.visualLineProjection.visualLineIndexAt(selection.active);
		const visualLine = context.visualLineProjection.lineAt(visualLineIndex);
		const renderedLine = context.renderedLines.get(visualLineIndex);
		if (!visualLine || !renderedLine) continue;
		const offset = selection.active.columnIndex - visualLine.startColumn;
		if (!isCurrentDomTextOffset(renderedLine.textElement, offset)) continue;
		const left = getStanzaDomTextCaretLeft(
			renderedLine.textElement,
			offset,
			renderedLine.domNode.domNode,
		);
		if (left === undefined) continue;
		caretIndexes.add(selectionIndex);
		domCarets.push(Object.freeze({
			selectionIndex,
			visualLineIndex,
			left,
			primary: selectionIndex === selections.primaryIndex,
		}));
	}
	if (selectionIndexes.size === 0 && caretIndexes.size === 0) return undefined;
	return Object.freeze({
		selectionIndexes,
		selections: Object.freeze(domSelections),
		caretIndexes,
		carets: Object.freeze(domCarets),
	});
}

export interface DomVisualRangeRectangle {
	readonly visualLineIndex: number;
	readonly left: number;
	readonly width: number;
}

export function createStanzaDomRangeRectangles(context: ViewportOverlayContext, range: TextRange): readonly DomVisualRangeRectangle[] | undefined {
	const result: DomVisualRangeRectangle[] = [];
	let intersectsRenderedLine = false;
	for (let visualLineIndex = context.renderLines.startLineIndex; visualLineIndex < context.renderLines.endLineIndexExclusive; visualLineIndex += 1) {
		const visualLine = context.visualLineProjection.lineAt(visualLineIndex);
		const renderedLine = context.renderedLines.get(visualLineIndex);
		if (!visualLine || !renderedLine || visualLine.logicalLineIndex < range.start.lineIndex || visualLine.logicalLineIndex > range.end.lineIndex) continue;
		const startColumn = visualLine.logicalLineIndex === range.start.lineIndex
			? Math.max(visualLine.startColumn, range.start.columnIndex)
			: visualLine.startColumn;
		const endColumn = visualLine.logicalLineIndex === range.end.lineIndex
			? Math.min(visualLine.endColumn, range.end.columnIndex)
			: visualLine.endColumn;
		if (endColumn <= startColumn) continue;
		intersectsRenderedLine = true;
		const startOffset = startColumn - visualLine.startColumn;
		const endOffset = endColumn - visualLine.startColumn;
		if (!isCurrentDomTextOffset(renderedLine.textElement, startOffset) || !isCurrentDomTextOffset(renderedLine.textElement, endOffset)) return undefined;
		const rectangles = getStanzaDomTextRangeRectangles(
			renderedLine.textElement,
			startOffset,
			endOffset,
			renderedLine.domNode.domNode,
		);
		if (!rectangles) return undefined;
		result.push(...rectangles.map(rectangle => Object.freeze({
			visualLineIndex,
			left: rectangle.left,
			width: rectangle.width,
		})));
	}
	return intersectsRenderedLine ? Object.freeze(result) : undefined;
}

function isCurrentDomTextOffset(element: HTMLElement, offset: number): boolean {
	return Number.isSafeInteger(offset) && offset >= 0 && offset <= element.textContent?.length;
}
