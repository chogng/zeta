import { EditorCommandHistoryMode, type EditorEditCommand } from "../commands/editorEditCommand.js";
import { createSelectionEditCommand, normalizeSelectionOffsets, type EditorSelectionEdit } from "./cursorTypeEditOperations.js";
import { type TextSelectionSet } from "../core/selection.js";
import { TextPosition, TextRange } from "../core/text.js";
import { type TextModel } from "../model/textModel.js";
import { nextCursorAtomicPosition, previousCursorAtomicPosition } from "./cursorAtomicMoveOperations.js";

/** Deletes the ranges selected by the clipboard owner as one isolated transaction. */
export function createCutCommand(model: TextModel, selections: TextSelectionSet, cutRanges: readonly TextRange[]): EditorEditCommand {
	if (cutRanges.length !== selections.selections.length) {
		throw new TypeError("Cut ranges must match the editor selections");
	}
	const sourceRanges = mergeDeletionRanges(model, cutRanges);
	const selectionsAfter = normalizeSelectionOffsets(cutRanges.map(range => {
		const targetOffset = mapOffsetThroughDeletions(model.offsetAt(range.start), sourceRanges);
		return {
			anchorOffset: targetOffset,
			activeOffset: targetOffset,
		};
	}), selections.primaryIndex);
	return {
		edits: Object.freeze(sourceRanges.map(range => ({ range: range.range, text: "" }))),
		selectionsAfter: selectionsAfter.selections,
		primarySelectionIndex: selectionsAfter.primaryIndex,
		historyMode: EditorCommandHistoryMode.Isolated,
	};
}

/** Deletes each selection or the preceding grapheme/newline. */
export function createBackspaceCommand(model: TextModel, selections: TextSelectionSet): EditorEditCommand {
	return createSelectionEditCommand(
		model,
		selections,
		selections.selections.map(selection => emptySelectionEdit(
			selection.collapsed
				? getPreviousDeleteRange(model, selection.active)
				: selection.range,
		)),
		EditorCommandHistoryMode.CoalesceBackspace,
	);
}

/** Deletes each selection or the following grapheme/newline. */
export function createDeleteForwardCommand(model: TextModel, selections: TextSelectionSet): EditorEditCommand {
	return createSelectionEditCommand(
		model,
		selections,
		selections.selections.map(selection => emptySelectionEdit(
			selection.collapsed
				? nextDeleteRange(model, selection.active)
				: selection.range,
		)),
		EditorCommandHistoryMode.CoalesceDelete,
	);
}

/** Deletes each selection or the text from its cursor back to the physical line start. */
export function createDeleteToLineStartCommand(model: TextModel, selections: TextSelectionSet): EditorEditCommand {
	return createDeleteToLineBoundaryCommand(model, selections, "start");
}

/** Deletes each selection or the text from its cursor through the physical line end. */
export function createDeleteToLineEndCommand(model: TextModel, selections: TextSelectionSet): EditorEditCommand {
	return createDeleteToLineBoundaryCommand(model, selections, "end");
}

export function getPreviousDeleteRange(model: TextModel, position: TextPosition): TextRange {
	return TextRange.from(previousCursorAtomicPosition(model, position), position);
}

function createDeleteToLineBoundaryCommand(model: TextModel, selections: TextSelectionSet, boundary: "start" | "end"): EditorEditCommand {
	return createSelectionEditCommand(
		model,
		selections,
		selections.selections.map(selection => {
			const range = selection.collapsed
				? boundary === "start"
					? TextRange.from(TextPosition.at(selection.active.lineIndex, 0), selection.active)
					: TextRange.from(selection.active, TextPosition.at(
						selection.active.lineIndex,
						model.getLineContent(selection.active.lineIndex).length,
					))
				: selection.range;
			return emptySelectionEdit(range);
		}),
		EditorCommandHistoryMode.Isolated,
	);
}

function emptySelectionEdit(range: TextRange): EditorSelectionEdit {
	return {
		range,
		text: "",
		anchorOffsetInText: 0,
		activeOffsetInText: 0,
	};
}

function nextDeleteRange(model: TextModel, position: TextPosition): TextRange {
	return TextRange.from(position, nextCursorAtomicPosition(model, position));
}

interface OffsetDeletionRange {
	readonly range: TextRange;
	readonly startOffset: number;
	readonly endOffset: number;
}

function mergeDeletionRanges(model: TextModel, ranges: readonly TextRange[]): readonly OffsetDeletionRange[] {
	const sorted = ranges.map(range => ({
		startOffset: model.offsetAt(range.start),
		endOffset: model.offsetAt(range.end),
	})).filter(range => range.startOffset !== range.endOffset).sort((left, right) => left.startOffset - right.startOffset || left.endOffset - right.endOffset);
	const merged: Array<{ startOffset: number; endOffset: number }> = [];
	for (const range of sorted) {
		const previous = merged[merged.length - 1];
		if (previous && range.startOffset <= previous.endOffset) {
			previous.endOffset = Math.max(previous.endOffset, range.endOffset);
		} else {
			merged.push({ ...range });
		}
	}
	return Object.freeze(merged.map(range => Object.freeze({
		...range,
		range: TextRange.from(model.positionAt(range.startOffset), model.positionAt(range.endOffset)),
	})));
}

function mapOffsetThroughDeletions(offset: number, ranges: readonly OffsetDeletionRange[]): number {
	let delta = 0;
	for (const range of ranges) {
		if (offset < range.startOffset) break;
		if (offset <= range.endOffset) return range.startOffset + delta;
		delta -= range.endOffset - range.startOffset;
	}
	return offset + delta;
}
