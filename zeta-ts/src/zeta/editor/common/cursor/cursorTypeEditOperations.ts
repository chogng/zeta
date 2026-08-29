import { EditorCommandHistoryMode, type EditorEditCommand, type TextSelectionOffsets } from "../commands/editorEditCommand.js";
import { type TextSelectionSet } from "../core/selection.js";
import { normalizeTextLineEndings, TextRange, type TextEdit } from "../core/text.js";
import { type TextModel } from "../model/textModel.js";
import { getTextGraphemeBoundaries } from "../core/textSegmentation.js";
import { advanceCursorAtomicPositionInLine } from "./cursorAtomicMoveOperations.js";

interface SelectionReplacement {
	readonly selectionIndex: number;
	readonly range: TextRange;
	readonly startOffset: number;
	readonly endOffset: number;
	readonly text: string;
	readonly anchorOffsetInText: number;
	readonly activeOffsetInText: number;
}

export interface EditorSelectionEdit {
	readonly range: TextRange;
	readonly text: string;
	readonly anchorOffsetInText: number;
	readonly activeOffsetInText: number;
}

/** Builds one validated multi-selection command from pre-change replacement scripts. */
export function createSelectionEditCommand(model: TextModel, selections: TextSelectionSet, edits: readonly EditorSelectionEdit[], historyMode: EditorCommandHistoryMode): EditorEditCommand {
	if (!Array.isArray(edits) || edits.length !== selections.selections.length) {
		throw new RangeError("Selection edits must match the selection count");
	}
	return buildSelectionEditCommand(
		model,
		selections,
		edits.map((edit, selectionIndex) => {
			if (typeof edit !== "object" || edit === null || typeof edit.text !== "string") {
				throw new TypeError("Selection edit must contain replacement text");
			}
			if (!Number.isSafeInteger(edit.anchorOffsetInText) || edit.anchorOffsetInText < 0 || !Number.isSafeInteger(edit.activeOffsetInText) || edit.activeOffsetInText < 0) {
				throw new RangeError("Selection edit result offsets must be non-negative safe integers");
			}
			return {
				selectionIndex,
				range: edit.range,
				startOffset: model.offsetAt(edit.range.start),
				endOffset: model.offsetAt(edit.range.end),
				text: edit.text,
				anchorOffsetInText: edit.anchorOffsetInText,
				activeOffsetInText: edit.activeOffsetInText,
			};
		}),
		historyMode,
	);
}

export function normalizeSelectionOffsets(selections: readonly TextSelectionOffsets[], primaryIndex: number): {
	readonly selections: readonly TextSelectionOffsets[];
	readonly primaryIndex: number;
} {
	const normalized: TextSelectionOffsets[] = [];
	const sourceToNormalized: number[] = [];
	for (const selection of selections) {
		let targetIndex = normalized.findIndex(candidate =>
			candidate.anchorOffset === selection.anchorOffset &&
			candidate.activeOffset === selection.activeOffset
		);
		if (targetIndex < 0) {
			targetIndex = normalized.length;
			normalized.push(selection);
		}
		sourceToNormalized.push(targetIndex);
	}
	return {
		selections: Object.freeze(normalized),
		primaryIndex: sourceToNormalized[primaryIndex]!,
	};
}

function buildSelectionEditCommand(
	model: TextModel,
	selections: TextSelectionSet,
	replacements: readonly SelectionReplacement[],
	historyMode: EditorCommandHistoryMode,
): EditorEditCommand {
	const sorted = [...replacements].sort((left, right) =>
		left.startOffset - right.startOffset ||
		left.endOffset - right.endOffset ||
		left.selectionIndex - right.selectionIndex
	);
	validateNonOverlapping(sorted);
	const selectionsAfter = new Array<{
		readonly anchorOffset: number;
		readonly activeOffset: number;
	}>(selections.selections.length);
	const edits: TextEdit[] = [];
	let cumulativeDelta = 0;
	for (const item of sorted) {
		selectionsAfter[item.selectionIndex] = {
			anchorOffset: item.startOffset + cumulativeDelta + item.anchorOffsetInText,
			activeOffset: item.startOffset + cumulativeDelta + item.activeOffsetInText,
		};
		if (item.startOffset !== item.endOffset || item.text.length > 0) {
			edits.push({ range: item.range, text: item.text });
		}
		cumulativeDelta +=
			item.text.length -
			(item.endOffset - item.startOffset);
	}
	const normalizedSelections = normalizeSelectionOffsets(
		selectionsAfter,
		selections.primaryIndex,
	);
	return {
		edits: Object.freeze(edits),
		selectionsAfter: normalizedSelections.selections,
		primarySelectionIndex: normalizedSelections.primaryIndex,
		historyMode,
	};
}

function validateNonOverlapping(replacements: readonly SelectionReplacement[]): void {
	for (let index = 1; index < replacements.length; index += 1) {
		const previous = replacements[index - 1]!;
		const current = replacements[index]!;
		const ambiguousSharedStart =
			current.startOffset === previous.startOffset &&
			(
				current.startOffset === current.endOffset ||
				previous.startOffset === previous.endOffset
			);
		if (
			current.startOffset < previous.endOffset ||
			ambiguousSharedStart
		) {
			throw new RangeError(
				"Selections must not overlap when creating an edit command",
			);
		}
	}
}

export function createOvertypeTextCommand(model: TextModel, selections: TextSelectionSet, text: string): EditorEditCommand {
	if (typeof text !== "string") throw new TypeError("Overtype text must be a string");
	const normalized = normalizeTextLineEndings(text);
	const graphemeCount = normalized.includes("\n") ? 0 : getTextGraphemeBoundaries(normalized).length - 1;
	return createSelectionEditCommand(model, selections, selections.selections.map(selection => {
		const range = selection.collapsed && graphemeCount > 0
			? TextRange.from(selection.active, advanceCursorAtomicPositionInLine(model, selection.active, graphemeCount))
			: selection.range;
		return { range, text: normalized, anchorOffsetInText: normalized.length, activeOffsetInText: normalized.length };
	}), EditorCommandHistoryMode.CoalesceTyping);
}
