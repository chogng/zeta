import { EditorCursorNavigationCommand, EditorCursorNavigationMode, MoveOperations } from './cursorMoveOperations.js';
import { EditorCommandHistoryMode, type EditorEditCommand } from "../commands/editorEditCommand.js";
import { TypeWithoutInterceptorsOperation, type SelectionEdit } from './cursorTypeEditOperations.js';
import { type Selection } from "../core/selection.js";
import { Position } from "../core/position.js";
import { Range } from "../core/range.js";
import { type TextModel } from "../model/textModel.js";
import { getTextWordSegments } from "../core/textSegmentation.js";

/** Deletes each selection or the preceding editor word segment. */
export class WordOperations {
	public static deleteWordLeft(model: TextModel, selections: readonly Selection[], wordPattern?: RegExp): EditorEditCommand {
		return createDeleteWordCommand(model, selections, EditorCursorNavigationCommand.WordLeft, EditorCommandHistoryMode.CoalesceBackspace, wordPattern);
	}

/** Deletes each selection or the following editor word segment. */
	public static deleteWordRight(model: TextModel, selections: readonly Selection[], wordPattern?: RegExp): EditorEditCommand {
		return createDeleteWordCommand(model, selections, EditorCursorNavigationCommand.WordRight, EditorCommandHistoryMode.CoalesceDelete, wordPattern);
	}

	public static getWordSelectionRange(model: TextModel, position: Position, wordPattern?: RegExp): Range {
		model.offsetAt(position);
		const line = model.getLineContent(position.lineNumber);
		if (line.length === 0) return Range.fromPositions(position);
		const probe = position.column === line.length + 1 ? line.length - 1 : position.column - 1;
		const patternRange = wordPatternRange(line, probe, wordPattern);
		if (patternRange) return Range.fromPositions(new Position(position.lineNumber, patternRange.start + 1), new Position(position.lineNumber, patternRange.end + 1));
		const segment = getTextWordSegments(line).find(candidate => probe >= candidate.start && probe < candidate.end);
		if (!segment) throw new RangeError('Word-selection probe is outside the line');
		return Range.fromPositions(new Position(position.lineNumber, segment.start + 1), new Position(position.lineNumber, segment.end + 1));
	}

	public static getTextWordRanges(text: string, wordPattern?: RegExp): readonly { readonly start: number; readonly end: number }[] {
		if (wordPattern) return Object.freeze(wordPatternRanges(text, wordPattern));
		return Object.freeze(getTextWordSegments(text).flatMap(segment => segment.wordLike ? [{ start: segment.start, end: segment.end }] : []));
	}
}

function createDeleteWordCommand(model: TextModel, selections: readonly Selection[], navigation: EditorCursorNavigationCommand, historyMode: EditorCommandHistoryMode, wordPattern: RegExp | undefined): EditorEditCommand {
	return TypeWithoutInterceptorsOperation.getEdits(
		model,
		selections,
		selections.map(selection => {
			const range = selection.isEmpty()
				? MoveOperations.navigate(model, [selection], {
					command: navigation,
					mode: EditorCursorNavigationMode.Extend,
					...(wordPattern ? { wordPattern } : {}),
				}).selections[0]!
				: selection;
			return emptySelectionEdit(range);
		}),
		historyMode,
	);
}

function emptySelectionEdit(range: Range): SelectionEdit {
	return {
		range: new Range(
			range.startLineNumber,
			range.startColumn,
			range.endLineNumber,
			range.endColumn,
		),
		text: "",
		anchorOffsetInText: 0,
		activeOffsetInText: 0,
	};
}

function wordPatternRange(line: string, probe: number, pattern: RegExp | undefined): { readonly start: number; readonly end: number } | undefined {
	return pattern ? wordPatternRanges(line, pattern).find(range => probe >= range.start && probe < range.end) : undefined;
}

function wordPatternRanges(line: string, pattern: RegExp): readonly { readonly start: number; readonly end: number }[] {
	const flags = pattern.flags.replaceAll("y", "").includes("g") ? pattern.flags.replaceAll("y", "") : `${pattern.flags.replaceAll("y", "")}g`;
	const matcher = new RegExp(pattern.source, flags);
	const ranges: { start: number; end: number }[] = [];
	for (let match = matcher.exec(line); match; match = matcher.exec(line)) {
		if (match[0].length === 0) {
			matcher.lastIndex += 1;
			continue;
		}
		ranges.push({ start: match.index, end: match.index + match[0].length });
	}
	return ranges;
}
