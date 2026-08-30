import { EditorCommandHistoryMode, type EditorEditCommand } from "../../../common/commands/editorEditCommand.js";
import { Selection } from "../../../common/core/selection.js";
import { SelectionSet } from "../../../common/cursor/selectionSet.js";
import { Position } from "../../../common/core/position.js";
import { Range } from "../../../common/core/range.js";

import { type TextModel } from "../../../common/model/textModel.js";
import { type TextEdit } from '../../../common/languages.js';

export enum EditorLineDuplicateDirection {
	Up = "up",
	Down = "down",
}

export enum EditorLineMoveDirection {
	Up = "up",
	Down = "down",
}

/** Selects whether a blank line is inserted before or after selected line groups. */
export enum EditorLineInsertDirection {
	Before = "before",
	After = "after",
}

interface OffsetEdit {
	readonly startOffset: number;
	readonly endOffset: number;
	readonly text: string;
	readonly edit: TextEdit;
}

/** Deletes the union of physical lines selected by every cursor. */
export function createDeleteLinesCommand(model: TextModel, selections: SelectionSet): EditorEditCommand {
	const groups = contiguousLineGroups(selectedLineIndices(selections));
	const edits = groups.flatMap<OffsetEdit>(group => deleteLineGroup(model, group));
	return createLineOperationCommand(model, selections, edits);
}

/** Duplicates the union of physical lines selected by every cursor. */
export function createDuplicateLinesCommand(model: TextModel, selections: SelectionSet, direction: EditorLineDuplicateDirection): EditorEditCommand {
	if (!Object.values(EditorLineDuplicateDirection).includes(direction)) {
		throw new TypeError("Unknown editor line duplicate direction");
	}
	const groups = contiguousLineGroups(selectedLineIndices(selections));
	const edits = groups.flatMap<OffsetEdit>(group => duplicateLineGroup(model, group, direction));
	return createLineOperationCommand(model, selections, edits);
}

/** Moves the union of selected physical lines by one neighboring line. */
export function createMoveLinesCommand(model: TextModel, selections: SelectionSet, direction: EditorLineMoveDirection): EditorEditCommand {
	if (!Object.values(EditorLineMoveDirection).includes(direction)) {
		throw new TypeError("Unknown editor line move direction");
	}
	const groups = contiguousLineGroups(selectedLineIndices(selections));
	const movableGroups = groups.filter(group => direction === EditorLineMoveDirection.Up
		? group.startLineIndex > 0
		: group.endLineIndex + 1 < model.lineCount);
	const edits = movableGroups.map(group => moveLineGroup(model, group, direction));
	const finalText = applyOffsetEdits(model.createSnapshot().getText(), edits);
	return Object.freeze({
		edits: Object.freeze(edits.map(edit => edit.edit)),
		selectionsAfter: Object.freeze(selections.selections.map(selection => Object.freeze({
			anchorOffset: offsetInText(finalText, movePosition(selection.getSelectionStart(), movableGroups, direction)),
			activeOffset: offsetInText(finalText, movePosition(selection.getPosition(), movableGroups, direction)),
		}))),
		primarySelectionIndex: selections.primaryIndex,
		historyMode: EditorCommandHistoryMode.Isolated,
	});
}

/** Inserts one blank line adjacent to each contiguous selected physical-line group. */
export function createInsertLineCommand(model: TextModel, selections: SelectionSet, direction: EditorLineInsertDirection): EditorEditCommand {
	if (!Object.values(EditorLineInsertDirection).includes(direction)) {
		throw new TypeError("Unknown editor line insertion direction");
	}
	const groups = contiguousLineGroups(selectedLineIndices(selections));
	const edits = groups.map(group => insertLineAtGroup(model, group, direction));
	const finalText = applyOffsetEdits(model.createSnapshot().getText(), edits);
	const nextSelections = groups.map((group, index) => Selection.fromPositions(new Position((insertedLineIndex(group, index, direction)) + 1, (0) + 1)));
	const primaryIndex = primaryInsertedGroupIndex(selections, groups);
	return Object.freeze({
		edits: Object.freeze(edits.map(edit => edit.edit)),
		selectionsAfter: Object.freeze(nextSelections.map(selection => Object.freeze({
			anchorOffset: offsetInText(finalText, selection.getSelectionStart()),
			activeOffset: offsetInText(finalText, selection.getPosition()),
		}))),
		primarySelectionIndex: primaryIndex,
		historyMode: EditorCommandHistoryMode.Isolated,
	});
}

function deleteLineGroup(model: TextModel, group: EditorLineGroup): readonly OffsetEdit[] {
	const first = group.startLineIndex;
	const last = group.endLineIndex;
	if (first === 0 && last === model.lineCount - 1) {
		const start = new Position((0) + 1, (0) + 1);
		const end = new Position((last) + 1, (model.getLineContent((last) + 1).length) + 1);
		return [offsetEdit(model, start, end, "")];
	}
	if (last + 1 < model.lineCount) {
		return [offsetEdit(model, new Position((first) + 1, (0) + 1), new Position((last + 1) + 1, (0) + 1), "")];
	}
	const previousLineIndex = first - 1;
	return [offsetEdit(
		model,
		new Position((previousLineIndex) + 1, (model.getLineContent((previousLineIndex) + 1).length) + 1),
		new Position((last) + 1, (model.getLineContent((last) + 1).length) + 1),
		"",
	)];
}

function duplicateLineGroup(model: TextModel, group: EditorLineGroup, direction: EditorLineDuplicateDirection): readonly OffsetEdit[] {
	const text = Array.from(
		{ length: group.endLineIndex - group.startLineIndex + 1 },
		(_, index) => model.getLineContent((group.startLineIndex + index) + 1),
	).join("\n");
	if (direction === EditorLineDuplicateDirection.Up) {
		return [offsetEdit(model, new Position((group.startLineIndex) + 1, (0) + 1), new Position((group.startLineIndex) + 1, (0) + 1), `${text}\n`)];
	}
	if (group.endLineIndex + 1 < model.lineCount) {
		return [offsetEdit(model, new Position((group.endLineIndex + 1) + 1, (0) + 1), new Position((group.endLineIndex + 1) + 1, (0) + 1), `${text}\n`)];
	}
	const end = new Position((group.endLineIndex) + 1, (model.getLineContent((group.endLineIndex) + 1).length) + 1);
	return [offsetEdit(model, end, end, `\n${text}`)];
}

function moveLineGroup(model: TextModel, group: EditorLineGroup, direction: EditorLineMoveDirection): OffsetEdit {
	if (direction === EditorLineMoveDirection.Up) {
		const previousLineIndex = group.startLineIndex - 1;
		const start = new Position((previousLineIndex) + 1, (0) + 1);
		const end = new Position((group.endLineIndex) + 1, (model.getLineContent((group.endLineIndex) + 1).length) + 1);
		const previous = model.getLineContent((previousLineIndex) + 1);
		const selected = lineGroupText(model, group);
		return offsetEdit(model, start, end, `${selected}\n${previous}`);
	}
	const nextLineIndex = group.endLineIndex + 1;
	const start = new Position((group.startLineIndex) + 1, (0) + 1);
	const end = new Position((nextLineIndex) + 1, (model.getLineContent((nextLineIndex) + 1).length) + 1);
	const selected = lineGroupText(model, group);
	const next = model.getLineContent((nextLineIndex) + 1);
	return offsetEdit(model, start, end, `${next}\n${selected}`);
}

function insertLineAtGroup(model: TextModel, group: EditorLineGroup, direction: EditorLineInsertDirection): OffsetEdit {
	if (direction === EditorLineInsertDirection.Before) {
		const position = new Position((group.startLineIndex) + 1, (0) + 1);
		return offsetEdit(model, position, position, "\n");
	}
	const lineIndex = group.endLineIndex + 1;
	const position = lineIndex < model.lineCount
		? new Position((lineIndex) + 1, (0) + 1)
		: new Position((group.endLineIndex) + 1, (model.getLineContent((group.endLineIndex) + 1).length) + 1);
	return offsetEdit(model, position, position, "\n");
}

function insertedLineIndex(group: EditorLineGroup, precedingInsertions: number, direction: EditorLineInsertDirection): number {
	return direction === EditorLineInsertDirection.Before
		? group.startLineIndex + precedingInsertions
		: group.endLineIndex + precedingInsertions + 1;
}

function primaryInsertedGroupIndex(selections: SelectionSet, groups: readonly EditorLineGroup[]): number {
	const primaryLines = selectedLineIndices(SelectionSet.single(selections.primary));
	for (const lineIndex of primaryLines) {
		const groupIndex = groups.findIndex(group =>
			lineIndex >= group.startLineIndex && lineIndex <= group.endLineIndex
		);
		if (groupIndex >= 0) return groupIndex;
	}
	return 0;
}

function lineGroupText(model: TextModel, group: EditorLineGroup): string {
	return Array.from(
		{ length: group.endLineIndex - group.startLineIndex + 1 },
		(_, index) => model.getLineContent((group.startLineIndex + index) + 1),
	).join("\n");
}

function createLineOperationCommand(model: TextModel, selections: SelectionSet, edits: readonly OffsetEdit[]): EditorEditCommand {
	return Object.freeze({
		edits: Object.freeze(edits.map(edit => edit.edit)),
		selectionsAfter: Object.freeze(selections.selections.map(selection => Object.freeze({
			anchorOffset: mapOffsetThroughEdits(model.offsetAt(selection.getSelectionStart()), edits),
			activeOffset: mapOffsetThroughEdits(model.offsetAt(selection.getPosition()), edits),
		}))),
		primarySelectionIndex: selections.primaryIndex,
		historyMode: EditorCommandHistoryMode.Isolated,
	});
}

function offsetEdit(model: TextModel, start: Position, end: Position, text: string): OffsetEdit {
	return Object.freeze({
		startOffset: model.offsetAt(start),
		endOffset: model.offsetAt(end),
		text,
		edit: Object.freeze({ range: Range.fromPositions(start, end), text }),
	});
}

interface EditorLineGroup {
	readonly startLineIndex: number;
	readonly endLineIndex: number;
}

function selectedLineIndices(selections: SelectionSet): readonly number[] {
	const indices = new Set<number>();
	for (const selection of selections.selections) {
		const range = selection;
		let endLineIndex = range.endLineNumber - 1;
		if (!selection.isEmpty() && range.endColumn === 1 && endLineIndex > range.startLineNumber - 1) {
			endLineIndex -= 1;
		}
		for (let lineIndex = range.startLineNumber - 1; lineIndex <= endLineIndex; lineIndex += 1) indices.add(lineIndex);
	}
	return Object.freeze([...indices].sort((left, right) => left - right));
}

function contiguousLineGroups(lineIndices: readonly number[]): readonly EditorLineGroup[] {
	const groups: EditorLineGroup[] = [];
	for (const lineIndex of lineIndices) {
		const previous = groups.at(-1);
		if (previous && lineIndex === previous.endLineIndex + 1) {
			groups[groups.length - 1] = Object.freeze({ ...previous, endLineIndex: lineIndex });
		} else {
			groups.push(Object.freeze({ startLineIndex: lineIndex, endLineIndex: lineIndex }));
		}
	}
	return Object.freeze(groups);
}

function mapOffsetThroughEdits(offset: number, edits: readonly OffsetEdit[]): number {
	let delta = 0;
	for (const edit of edits) {
		if (offset < edit.startOffset) break;
		if (edit.startOffset === edit.endOffset && offset === edit.startOffset) {
			return offset + delta + edit.text.length;
		}
		if (offset <= edit.endOffset) {
			return edit.startOffset + delta + Math.min(offset - edit.startOffset, edit.text.length);
		}
		delta += edit.text.length - (edit.endOffset - edit.startOffset);
	}
	return offset + delta;
}

function movePosition(position: Position, groups: readonly EditorLineGroup[], direction: EditorLineMoveDirection): Position {
	const lineIndex = position.lineNumber - 1;
	const group = groups.find(candidate => lineIndex >= candidate.startLineIndex && lineIndex <= candidate.endLineIndex);
	if (!group) return position;
	return new Position(position.lineNumber + (direction === EditorLineMoveDirection.Up ? -1 : 1), position.column);
}

function applyOffsetEdits(text: string, edits: readonly OffsetEdit[]): string {
	let result = text;
	for (let index = edits.length - 1; index >= 0; index -= 1) {
		const edit = edits[index]!;
		result = result.slice(0, edit.startOffset) + edit.text + result.slice(edit.endOffset);
	}
	return result;
}

function offsetInText(text: string, position: Position): number {
	let lineIndex = 0;
	let offset = 0;
	while (lineIndex < position.lineNumber - 1) {
		const next = text.indexOf("\n", offset);
		if (next < 0) throw new RangeError("Moved line position is outside the result text");
		offset = next + 1;
		lineIndex += 1;
	}
	const lineEnd = text.indexOf("\n", offset);
	const length = (lineEnd < 0 ? text.length : lineEnd) - offset;
	if (position.column < 1 || position.column > length + 1) throw new RangeError("Moved line position exceeds its result line");
	return offset + position.column - 1;
}
