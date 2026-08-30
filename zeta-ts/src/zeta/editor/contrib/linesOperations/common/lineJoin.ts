import { EditorCommandHistoryMode, type EditorEditCommand, type TextSelectionOffsets } from "../../../common/commands/editorEditCommand.js";
import { type Selection } from "../../../common/core/selection.js";
import { Position } from "../../../common/core/position.js";
import { Range } from "../../../common/core/range.js";

import { type TextModel } from "../../../common/model/textModel.js";
import { type TextEdit } from '../../../common/languages.js';

interface JoinSelection {
	readonly start: Position;
	readonly end: Position;
	readonly containsPrimary: boolean;
}

interface JoinOperation {
	readonly selection: JoinSelection;
	readonly range: Range;
	readonly startOffset: number;
	readonly endOffset: number;
	readonly replacement: string;
	readonly resultStartColumn: number;
	readonly resultEndColumn: number;
}

/**
 * Joins the physical lines covered by each cursor or selection in one isolated
 * edit. Leading indentation on subsequent non-empty lines is removed and one
 * separating space is retained when both adjacent fragments contain text.
 */
export function createJoinLinesCommand(model: TextModel, selections: readonly Selection[]): EditorEditCommand {
	const reduced = reduceJoinSelections(selections);
	const operations = reduced.map(selection => createJoinOperation(model, selection));
	if (operations.every(operation => operation.startOffset === operation.endOffset)) {
		return unchangedCommand(model, selections);
	}

	const edits: TextEdit[] = [];
	const selectionsAfter: TextSelectionOffsets[] = [];
	let primarySelectionIndex = 0;
	let cumulativeDelta = 0;
	for (const operation of operations) {
		const resultOffset = operation.startOffset + cumulativeDelta;
		selectionsAfter.push(Object.freeze({
			anchorOffset: resultOffset + operation.resultStartColumn,
			activeOffset: resultOffset + operation.resultEndColumn,
		}));
		if (operation.selection.containsPrimary) {
			primarySelectionIndex = selectionsAfter.length - 1;
		}
		if (operation.startOffset !== operation.endOffset) {
			edits.push(Object.freeze({
				range: operation.range,
				text: operation.replacement,
			}));
			cumulativeDelta += operation.replacement.length - (operation.endOffset - operation.startOffset);
		}
	}
	return Object.freeze({
		edits: Object.freeze(edits),
		selectionsAfter: Object.freeze(selectionsAfter),
		primarySelectionIndex,
		historyMode: EditorCommandHistoryMode.Isolated,
	});
}

function reduceJoinSelections(selections: readonly Selection[]): readonly JoinSelection[] {
	const ordered = selections.map((selection, index) => Object.freeze({
		start: selection.getStartPosition(),
		end: selection.getEndPosition(),
		collapsed: selection.isEmpty(),
		containsPrimary: index === 0,
	})).sort((left, right) => Position.compare(left.start, right.start) || Position.compare(left.end, right.end));
	const reduced: JoinSelection[] = [];
	for (const current of ordered) {
		const previous = reduced.at(-1);
		if (!previous) {
			reduced.push(Object.freeze(current));
			continue;
		}
		const previousCollapsed = Position.compare(previous.start, previous.end) === 0;
		if (previousCollapsed && previous.end.lineNumber === current.start.lineNumber) {
			reduced[reduced.length - 1] = Object.freeze({
				start: current.start,
				end: current.end,
				containsPrimary: previous.containsPrimary || current.containsPrimary,
			});
			continue;
		}
		const separated = previousCollapsed
			? current.start.lineNumber > previous.end.lineNumber + 1
			: current.start.lineNumber > previous.end.lineNumber;
		if (separated) {
			reduced.push(Object.freeze(current));
			continue;
		}
		reduced[reduced.length - 1] = Object.freeze({
			start: previous.start,
			end: current.end,
			containsPrimary: previous.containsPrimary || current.containsPrimary,
		});
	}
	return Object.freeze(reduced);
}

function createJoinOperation(model: TextModel, selection: JoinSelection): JoinOperation {
	const joinsFollowingLine = selection.start.lineNumber === selection.end.lineNumber;
	const endLineNumber = joinsFollowingLine
		? Math.min(selection.start.lineNumber + 1, model.lineCount)
		: selection.end.lineNumber;
	if (endLineNumber === selection.start.lineNumber) {
		const lineStart = new Position(selection.start.lineNumber, 1);
		const startOffset = model.offsetAt(lineStart);
		return Object.freeze({
			selection,
			range: Range.fromPositions(lineStart),
			startOffset,
			endOffset: startOffset,
			replacement: "",
			resultStartColumn: selection.start.column - 1,
			resultEndColumn: selection.end.column - 1,
		});
	}
	const end = new Position(endLineNumber, model.getLineContent(endLineNumber).length + 1);
	const joined = joinLineContents(model, selection.start.lineNumber, endLineNumber);
	const selectionEndOffset = model.getLineContent(selection.end.lineNumber).length - (selection.end.column - 1);
	const endColumn = joinsFollowingLine
		? joined.text.length - joined.finalSegmentLength
		: joined.text.length - selectionEndOffset;
	return Object.freeze({
		selection: Object.freeze({ ...selection, end }),
		range: Range.fromPositions(new Position(selection.start.lineNumber, 1), end),
		startOffset: model.offsetAt(new Position(selection.start.lineNumber, 1)),
		endOffset: model.offsetAt(end),
		replacement: joined.text,
		resultStartColumn: joinsFollowingLine ? endColumn : selection.start.column - 1,
		resultEndColumn: endColumn,
	});
}

function joinLineContents(model: TextModel, startLineNumber: number, endLineNumber: number): { readonly text: string; readonly finalSegmentLength: number } {
	let text = model.getLineContent(startLineNumber);
	let finalSegmentLength = 0;
	for (let lineNumber = startLineNumber + 1; lineNumber <= endLineNumber; lineNumber += 1) {
		const nextLine = model.getLineContent(lineNumber);
		const trimmed = nextLine.replace(/^[\s\uFEFF\xA0]+/u, "");
		if (trimmed.length === 0) {
			finalSegmentLength = 0;
			continue;
		}
		let insertSpace = text.length > 0;
		if (insertSpace && /[\s\uFEFF\xA0]$/u.test(text)) {
			insertSpace = false;
			text = text.replace(/[\s\uFEFF\xA0]+$/u, " ");
		}
		text += `${insertSpace ? " " : ""}${trimmed}`;
		finalSegmentLength = trimmed.length + (insertSpace ? 1 : 0);
	}
	return Object.freeze({ text, finalSegmentLength });
}

function unchangedCommand(model: TextModel, selections: readonly Selection[]): EditorEditCommand {
	return Object.freeze({
		edits: Object.freeze([]),
		selectionsAfter: Object.freeze(selections.map(selection => Object.freeze({
			anchorOffset: model.offsetAt(selection.getSelectionStart()),
			activeOffset: model.offsetAt(selection.getPosition()),
		}))),
		primarySelectionIndex: 0,
		historyMode: EditorCommandHistoryMode.Isolated,
	});
}
