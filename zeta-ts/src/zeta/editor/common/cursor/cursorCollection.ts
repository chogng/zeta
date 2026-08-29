import { TextSelection, TextSelectionSet } from "../core/selection.js";
import { normalizeTextLineEndings, type TextEdit } from "../core/text.js";
import { TextModel } from "../model/textModel.js";
import { type TextSelectionOffsets } from "../commands/editorEditCommand.js";

export function calculateResultLength(
	model: TextModel,
	edits: readonly TextEdit[],
): number {
	let length = model.createSnapshot().length;
	for (const edit of edits) {
		const startOffset = model.offsetAt(edit.range.start);
		const endOffset = model.offsetAt(edit.range.end);
		length += normalizeTextLineEndings(edit.text).length -
			(endOffset - startOffset);
	}
	return length;
}

export function selectionSetFromOffsets(
	model: TextModel,
	selections: readonly TextSelectionOffsets[],
	primarySelectionIndex: number,
): TextSelectionSet {
	return TextSelectionSet.withPrimary(
		selections.map(selection => TextSelection.from(
			model.positionAt(selection.anchorOffset),
			model.positionAt(selection.activeOffset),
		)),
		primarySelectionIndex,
	);
}

export function validateSelectionOffsets(
	selections: readonly TextSelectionOffsets[],
	primarySelectionIndex: number,
	documentLength: number,
): void {
	if (selections.length === 0) {
		throw new RangeError("selectionsAfter must not be empty");
	}
	if (
		!Number.isSafeInteger(primarySelectionIndex) ||
		primarySelectionIndex < 0 ||
		primarySelectionIndex >= selections.length
	) {
		throw new RangeError(
			"primarySelectionIndex must identify selectionsAfter",
		);
	}
	for (const selection of selections) {
		assertOffset(
			selection.anchorOffset,
			documentLength,
			"anchorOffset",
		);
		assertOffset(
			selection.activeOffset,
			documentLength,
			"activeOffset",
		);
	}
}

export function validateSelectionSet(
	model: TextModel,
	selections: TextSelectionSet,
): void {
	for (const selection of selections.selections) {
		model.offsetAt(selection.anchor);
		model.offsetAt(selection.active);
	}
}

export function selectionSetsEqual(
	left: TextSelectionSet,
	right: TextSelectionSet,
): boolean {
	return left.primaryIndex === right.primaryIndex &&
		left.selections.length === right.selections.length &&
		left.selections.every((selection, index) => {
			const other = right.selections[index];
			return selection.anchor.compareTo(other.anchor) === 0 &&
				selection.active.compareTo(other.active) === 0;
		});
}

function assertOffset(
	offset: number,
	documentLength: number,
	name: string,
): void {
	if (
		!Number.isSafeInteger(offset) ||
		offset < 0 ||
		offset > documentLength
	) {
		throw new RangeError(
			`${name} must be between 0 and ${documentLength}`,
		);
	}
}
