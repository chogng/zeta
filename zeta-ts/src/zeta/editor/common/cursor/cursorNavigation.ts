import { clamp } from '../../../base/common/numbers.js';
import { Position } from '../core/position.js';
import { Selection } from '../core/selection.js';
import { getTextGraphemeBoundaries, getTextWordSegments } from '../core/textSegmentation.js';
import type { TextModel } from '../model/textModel.js';
import { AtomicTabMoveOperations, Direction } from './cursorAtomicMoveOperations.js';
import { SelectionSet } from './selectionSet.js';

export enum EditorCursorNavigationCommand {
	CharacterLeft = 'characterLeft',
	CharacterRight = 'characterRight',
	WordLeft = 'wordLeft',
	WordRight = 'wordRight',
	LineUp = 'lineUp',
	LineDown = 'lineDown',
	LineStart = 'lineStart',
	LineEnd = 'lineEnd',
	DocumentStart = 'documentStart',
	DocumentEnd = 'documentEnd',
	PageUp = 'pageUp',
	PageDown = 'pageDown',
}

export enum EditorCursorNavigationMode {
	Move = 'move',
	Extend = 'extend',
}

export interface EditorCursorNavigationRequest {
	readonly command: EditorCursorNavigationCommand;
	readonly mode: EditorCursorNavigationMode;
	readonly pageLineCount?: number;
	readonly preferredColumns?: readonly number[];
	readonly wordPattern?: RegExp;
	readonly atomicTabSize?: number;
}

export interface EditorCursorNavigationResult {
	readonly selections: SelectionSet;
	readonly preferredColumns: readonly number[] | undefined;
}

/** Applies Zeta's SelectionSet navigation command and preserves primary identity. */
export class CursorNavigation {
	public static navigate(model: TextModel, selections: SelectionSet, request: EditorCursorNavigationRequest): EditorCursorNavigationResult {
		validateRequest(model, selections, request);
		const vertical = isVerticalCommand(request.command);
		const preferredColumns = vertical ? resolvePreferredColumns(selections, request.preferredColumns) : undefined;
		const navigated = selections.selections.map((selection, index) => {
			const target = navigationTarget(model, selection, request, preferredColumns?.[index]);
			return request.mode === EditorCursorNavigationMode.Extend
				? Selection.fromPositions(selection.getSelectionStart(), target)
				: Selection.fromPositions(target);
		});
		return normalizeResult(navigated, selections.primaryIndex, preferredColumns);
	}
}

function navigationTarget(model: TextModel, selection: Selection, request: EditorCursorNavigationRequest, preferredColumn: number | undefined): Position {
	if (request.mode === EditorCursorNavigationMode.Move && !selection.isEmpty()) {
		if (request.command === EditorCursorNavigationCommand.CharacterLeft || request.command === EditorCursorNavigationCommand.WordLeft) return selection.getStartPosition();
		if (request.command === EditorCursorNavigationCommand.CharacterRight || request.command === EditorCursorNavigationCommand.WordRight) return selection.getEndPosition();
	}
	const active = selection.getPosition();
	switch (request.command) {
		case EditorCursorNavigationCommand.CharacterLeft: return previousCharacter(model, active, request.atomicTabSize);
		case EditorCursorNavigationCommand.CharacterRight: return nextCharacter(model, active, request.atomicTabSize);
		case EditorCursorNavigationCommand.WordLeft: return previousWord(model, active, request.wordPattern);
		case EditorCursorNavigationCommand.WordRight: return nextWord(model, active, request.wordPattern);
		case EditorCursorNavigationCommand.LineUp: return verticalTarget(model, active, -1, preferredColumn);
		case EditorCursorNavigationCommand.LineDown: return verticalTarget(model, active, 1, preferredColumn);
		case EditorCursorNavigationCommand.PageUp: return verticalTarget(model, active, -(request.pageLineCount ?? 1), preferredColumn);
		case EditorCursorNavigationCommand.PageDown: return verticalTarget(model, active, request.pageLineCount ?? 1, preferredColumn);
		case EditorCursorNavigationCommand.LineStart: return new Position(active.lineNumber, 1);
		case EditorCursorNavigationCommand.LineEnd: return new Position(active.lineNumber, model.getLineContent(active.lineNumber).length + 1);
		case EditorCursorNavigationCommand.DocumentStart: return new Position(1, 1);
		case EditorCursorNavigationCommand.DocumentEnd: return new Position(model.lineCount, model.getLineContent(model.lineCount).length + 1);
	}
}

function previousCharacter(model: TextModel, position: Position, atomicTabSize: number | undefined): Position {
	if (atomicTabSize !== undefined) {
		const column = AtomicTabMoveOperations.atomicPosition(model.getLineContent(position.lineNumber), position.column - 1, atomicTabSize, Direction.Left);
		if (column >= 0) return new Position(position.lineNumber, column + 1);
	}
	if (position.column === 1) return position.lineNumber === 1 ? position : new Position(position.lineNumber - 1, model.getLineMaxColumn(position.lineNumber - 1));
	return new Position(position.lineNumber, previousBoundary(getTextGraphemeBoundaries(model.getLineContent(position.lineNumber)), position.column - 1) + 1);
}

function nextCharacter(model: TextModel, position: Position, atomicTabSize: number | undefined): Position {
	if (atomicTabSize !== undefined) {
		const column = AtomicTabMoveOperations.atomicPosition(model.getLineContent(position.lineNumber), position.column - 1, atomicTabSize, Direction.Right);
		if (column >= 0) return new Position(position.lineNumber, column + 1);
	}
	const line = model.getLineContent(position.lineNumber);
	if (position.column === line.length + 1) return position.lineNumber < model.lineCount ? new Position(position.lineNumber + 1, 1) : position;
	return new Position(position.lineNumber, nextBoundary(getTextGraphemeBoundaries(line), position.column - 1) + 1);
}

function previousWord(model: TextModel, position: Position, wordPattern: RegExp | undefined): Position {
	for (let lineNumber = position.lineNumber; lineNumber >= 1; lineNumber--) {
		const limit = lineNumber === position.lineNumber ? position.column - 1 : Number.POSITIVE_INFINITY;
		const segments = getTextWordRanges(model.getLineContent(lineNumber), wordPattern);
		for (let index = segments.length - 1; index >= 0; index--) if (segments[index]!.start < limit) return new Position(lineNumber, segments[index]!.start + 1);
	}
	return new Position(1, 1);
}

function nextWord(model: TextModel, position: Position, wordPattern: RegExp | undefined): Position {
	for (let lineNumber = position.lineNumber; lineNumber <= model.lineCount; lineNumber++) {
		const limit = lineNumber === position.lineNumber ? position.column - 1 : -1;
		for (const segment of getTextWordRanges(model.getLineContent(lineNumber), wordPattern)) if (segment.start > limit) return new Position(lineNumber, segment.start + 1);
	}
	return new Position(model.lineCount, model.getLineMaxColumn(model.lineCount));
}

function verticalTarget(model: TextModel, position: Position, lineDelta: number, preferredColumn: number | undefined): Position {
	const lineNumber = clamp(position.lineNumber + lineDelta, 1, model.lineCount);
	if (lineNumber === position.lineNumber) return position;
	const column = Math.min(preferredColumn ?? position.column, model.getLineMaxColumn(lineNumber));
	return new Position(lineNumber, boundaryAtOrBefore(getTextGraphemeBoundaries(model.getLineContent(lineNumber)), column - 1) + 1);
}

function resolvePreferredColumns(selections: SelectionSet, preferredColumns: readonly number[] | undefined): readonly number[] {
	return preferredColumns?.length === selections.selections.length
		? Object.freeze([...preferredColumns])
		: Object.freeze(selections.selections.map(selection => selection.getPosition().column));
}

function normalizeResult(selections: readonly Selection[], primaryIndex: number, preferredColumns: readonly number[] | undefined): EditorCursorNavigationResult {
	const normalized: Selection[] = [];
	const normalizedColumns: number[] = [];
	const sourceToNormalized: number[] = [];
	for (let index = 0; index < selections.length; index++) {
		const selection = selections[index]!;
		let targetIndex = normalized.findIndex(candidate => candidate.equalsSelection(selection));
		if (targetIndex < 0) {
			targetIndex = normalized.length;
			normalized.push(selection);
			if (preferredColumns) normalizedColumns.push(preferredColumns[index]!);
		} else if (preferredColumns && index === primaryIndex) {
			normalizedColumns[targetIndex] = preferredColumns[index]!;
		}
		sourceToNormalized.push(targetIndex);
	}
	return Object.freeze({
		selections: SelectionSet.withPrimary(normalized, sourceToNormalized[primaryIndex]!),
		preferredColumns: preferredColumns ? Object.freeze(normalizedColumns) : undefined,
	});
}

function validateRequest(model: TextModel, selections: SelectionSet, request: EditorCursorNavigationRequest): void {
	if (!Object.values(EditorCursorNavigationCommand).includes(request.command)) throw new TypeError('Unknown editor cursor navigation command');
	if (!Object.values(EditorCursorNavigationMode).includes(request.mode)) throw new TypeError('Unknown editor cursor navigation mode');
	if (request.pageLineCount !== undefined && (!Number.isSafeInteger(request.pageLineCount) || request.pageLineCount < 1)) throw new RangeError('pageLineCount must be a positive safe integer');
	if (request.preferredColumns && (request.preferredColumns.length !== selections.selections.length || request.preferredColumns.some(column => !Number.isSafeInteger(column) || column < 1))) throw new RangeError('preferredColumns must match selections');
	if (request.atomicTabSize !== undefined && (!Number.isSafeInteger(request.atomicTabSize) || request.atomicTabSize < 1)) throw new RangeError('atomicTabSize must be a positive safe integer');
	for (const selection of selections.selections) {
		model.offsetAt(selection.getSelectionStart());
		model.offsetAt(selection.getPosition());
	}
}

function isVerticalCommand(command: EditorCursorNavigationCommand): boolean {
	return command === EditorCursorNavigationCommand.LineUp || command === EditorCursorNavigationCommand.LineDown || command === EditorCursorNavigationCommand.PageUp || command === EditorCursorNavigationCommand.PageDown;
}

function previousBoundary(boundaries: readonly number[], column: number): number {
	for (let index = boundaries.length - 1; index >= 0; index--) if (boundaries[index]! < column) return boundaries[index]!;
	return 0;
}

function nextBoundary(boundaries: readonly number[], column: number): number {
	return boundaries.find(boundary => boundary > column) ?? boundaries.at(-1)!;
}

function boundaryAtOrBefore(boundaries: readonly number[], column: number): number {
	for (let index = boundaries.length - 1; index >= 0; index--) if (boundaries[index]! <= column) return boundaries[index]!;
	return 0;
}

function getTextWordRanges(text: string, wordPattern: RegExp | undefined): readonly { readonly start: number; readonly end: number }[] {
	if (!wordPattern) return getTextWordSegments(text).flatMap(segment => segment.wordLike ? [{ start: segment.start, end: segment.end }] : []);
	const flags = wordPattern.flags.replaceAll('y', '').includes('g') ? wordPattern.flags.replaceAll('y', '') : `${wordPattern.flags.replaceAll('y', '')}g`;
	const matcher = new RegExp(wordPattern.source, flags);
	const ranges: Array<{ readonly start: number; readonly end: number }> = [];
	for (let match = matcher.exec(text); match; match = matcher.exec(text)) {
		if (match[0].length === 0) {
			matcher.lastIndex++;
			continue;
		}
		ranges.push({ start: match.index, end: match.index + match[0].length });
	}
	return ranges;
}
