import { clamp } from "../../../base/common/numbers.js";
import { Selection } from "../core/selection.js";
import { SelectionSet } from "./selectionSet.js";
import { Position } from "../core/position.js";
import { type TextModel } from "../model/textModel.js";
import { getTextGraphemeBoundaries, getTextWordSegments } from '../core/textSegmentation.js';
import { AtomicTabMoveOperations, Direction } from './cursorAtomicMoveOperations.js';

export enum EditorCursorNavigationCommand {
	CharacterLeft = "characterLeft",
	CharacterRight = "characterRight",
	WordLeft = "wordLeft",
	WordRight = "wordRight",
	LineUp = "lineUp",
	LineDown = "lineDown",
	LineStart = "lineStart",
	LineEnd = "lineEnd",
	DocumentStart = "documentStart",
	DocumentEnd = "documentEnd",
	PageUp = "pageUp",
	PageDown = "pageDown",
}

export enum EditorCursorNavigationMode {
	Move = "move",
	Extend = "extend",
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

/**
 * Applies one DOM-independent cursor navigation command to every selection.
 *
 * Vertical commands retain caller-owned preferred UTF-16 columns. Exact
 * duplicate results coalesce while preserving the primary selection mapping.
 */
export class MoveOperations {
	public static navigate(model: TextModel, selections: SelectionSet, request: EditorCursorNavigationRequest): EditorCursorNavigationResult {
		validateRequest(model, selections, request);
		const vertical = isVerticalCommand(request.command);
		const preferredColumns = vertical
			? resolvePreferredColumns(selections, request.preferredColumns)
			: undefined;
		const navigated = selections.selections.map((selection, index) => {
			const target = navigationTarget(
				model,
				selection,
				request.command,
				request.pageLineCount ?? 1,
				preferredColumns?.[index],
				request.mode,
				request.wordPattern,
				request.atomicTabSize,
			);
			return request.mode === EditorCursorNavigationMode.Extend
				? Selection.fromPositions(selection.getSelectionStart(), target)
				: Selection.fromPositions(target);
		});
		return normalizeResult(
			navigated,
			selections.primaryIndex,
			preferredColumns,
		);
	}

	public static leftPosition(model: TextModel, position: Position, atomicTabSize?: number): Position {
		const atomicColumn = atomicTabSize === undefined ? -1 : AtomicTabMoveOperations.atomicPosition(model.getLineContent(position.lineNumber), position.column - 1, atomicTabSize, Direction.Left);
		if (atomicColumn >= 0) return new Position(position.lineNumber, atomicColumn + 1);
		return previousCharacter(model, position);
	}

	public static rightPosition(model: TextModel, position: Position, atomicTabSize?: number): Position {
		const atomicColumn = atomicTabSize === undefined ? -1 : AtomicTabMoveOperations.atomicPosition(model.getLineContent(position.lineNumber), position.column - 1, atomicTabSize, Direction.Right);
		if (atomicColumn >= 0) return new Position(position.lineNumber, atomicColumn + 1);
		return nextCharacter(model, position);
	}
}

function navigationTarget(
	model: TextModel,
	selection: Selection,
	command: EditorCursorNavigationCommand,
	pageLineCount: number,
	preferredColumn: number | undefined,
	mode: EditorCursorNavigationMode,
	wordPattern: RegExp | undefined,
	requestAtomicTabSize: number | undefined,
): Position {
	if (
		mode === EditorCursorNavigationMode.Move &&
		!selection.isEmpty()
	) {
		if (
			command === EditorCursorNavigationCommand.CharacterLeft ||
			command === EditorCursorNavigationCommand.WordLeft
		) {
			return selection.getStartPosition();
		}
		if (
			command === EditorCursorNavigationCommand.CharacterRight ||
			command === EditorCursorNavigationCommand.WordRight
		) {
			return selection.getEndPosition();
		}
	}

	const active = selection.getPosition();
	switch (command) {
		case EditorCursorNavigationCommand.CharacterLeft:
			return MoveOperations.leftPosition(model, active, requestAtomicTabSize);
		case EditorCursorNavigationCommand.CharacterRight:
			return MoveOperations.rightPosition(model, active, requestAtomicTabSize);
		case EditorCursorNavigationCommand.WordLeft:
			return previousWord(model, active, wordPattern);
		case EditorCursorNavigationCommand.WordRight:
			return nextWord(model, active, wordPattern);
		case EditorCursorNavigationCommand.LineUp:
			return verticalTarget(model, active, -1, preferredColumn);
		case EditorCursorNavigationCommand.LineDown:
			return verticalTarget(model, active, 1, preferredColumn);
		case EditorCursorNavigationCommand.PageUp:
			return verticalTarget(model, active, -pageLineCount, preferredColumn);
		case EditorCursorNavigationCommand.PageDown:
			return verticalTarget(model, active, pageLineCount, preferredColumn);
		case EditorCursorNavigationCommand.LineStart:
			return new Position(active.lineNumber, 1);
		case EditorCursorNavigationCommand.LineEnd:
			return new Position(active.lineNumber, model.getLineContent(active.lineNumber).length + 1);
		case EditorCursorNavigationCommand.DocumentStart:
			return new Position((0) + 1, (0) + 1);
		case EditorCursorNavigationCommand.DocumentEnd: {
			const lineIndex = model.lineCount - 1;
			return new Position((lineIndex) + 1, (model.getLineContent((lineIndex) + 1).length) + 1);
		}
	}
}

function previousCharacter(model: TextModel, position: Position): Position {
	if (position.column === 1) {
		if (position.lineNumber === 1) return position;
		const previousLineNumber = position.lineNumber - 1;
		return new Position(previousLineNumber, model.getLineContent(previousLineNumber).length + 1);
	}
	const boundaries = getTextGraphemeBoundaries(
		model.getLineContent(position.lineNumber),
	);
	return new Position(position.lineNumber, previousBoundary(boundaries, position.column - 1) + 1);
}

function nextCharacter(model: TextModel, position: Position): Position {
	const line = model.getLineContent(position.lineNumber);
	if (position.column === line.length + 1) {
		return position.lineNumber < model.lineCount
			? new Position(position.lineNumber + 1, 1)
			: position;
	}
	return new Position(position.lineNumber, nextBoundary(getTextGraphemeBoundaries(line), position.column - 1) + 1);
}

function previousWord(model: TextModel, position: Position, wordPattern: RegExp | undefined): Position {
	for (let lineNumber = position.lineNumber; lineNumber >= 1; lineNumber -= 1) {
		const limit = lineNumber === position.lineNumber
			? position.column - 1
			: Number.POSITIVE_INFINITY;
		const segments = getTextWordRanges(model.getLineContent(lineNumber), wordPattern);
		for (let index = segments.length - 1; index >= 0; index -= 1) {
			const segment = segments[index]!;
			if (segment.start < limit) {
				return new Position(lineNumber, segment.start + 1);
			}
		}
	}
	return new Position((0) + 1, (0) + 1);
}

function nextWord(model: TextModel, position: Position, wordPattern: RegExp | undefined): Position {
	for (
		let lineNumber = position.lineNumber;
		lineNumber <= model.lineCount;
		lineNumber += 1
	) {
		const limit = lineNumber === position.lineNumber
			? position.column - 1
			: -1;
		for (const segment of getTextWordRanges(model.getLineContent(lineNumber), wordPattern)) {
			if (segment.start > limit) {
				return new Position(lineNumber, segment.start + 1);
			}
		}
	}
	const lineIndex = model.lineCount - 1;
	return new Position((lineIndex) + 1, (model.getLineContent((lineIndex) + 1).length) + 1);
}

function verticalTarget(model: TextModel, position: Position, lineDelta: number, preferredColumn: number | undefined): Position {
	const lineNumber = clamp(
		position.lineNumber + lineDelta,
		1,
		model.lineCount,
	);
	if (lineNumber === position.lineNumber) return position;
	const line = model.getLineContent(lineNumber);
	const column = Math.min(preferredColumn ?? position.column, line.length + 1);
	return new Position(lineNumber, boundaryAtOrBefore(getTextGraphemeBoundaries(line), column - 1) + 1);
}

function resolvePreferredColumns(selections: SelectionSet, preferredColumns: readonly number[] | undefined): readonly number[] {
	if (preferredColumns?.length === selections.selections.length) {
		return Object.freeze([...preferredColumns]);
	}
	return Object.freeze(
		selections.selections.map(selection => selection.getPosition().column),
	);
}

function normalizeResult(selections: readonly Selection[], primaryIndex: number, preferredColumns: readonly number[] | undefined): EditorCursorNavigationResult {
	const normalized: Selection[] = [];
	const normalizedColumns: number[] = [];
	const sourceToNormalized: number[] = [];
	for (let index = 0; index < selections.length; index += 1) {
		const selection = selections[index]!;
		let targetIndex = normalized.findIndex(candidate =>
			selectionsEqual(candidate, selection)
		);
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
		selections: SelectionSet.withPrimary(
			normalized,
			sourceToNormalized[primaryIndex]!,
		),
		preferredColumns: preferredColumns
			? Object.freeze(normalizedColumns)
			: undefined,
	});
}

function validateRequest(model: TextModel, selections: SelectionSet, request: EditorCursorNavigationRequest): void {
	if (!Object.values(EditorCursorNavigationCommand).includes(request.command)) {
		throw new TypeError("Unknown editor cursor navigation command");
	}
	if (!Object.values(EditorCursorNavigationMode).includes(request.mode)) {
		throw new TypeError("Unknown editor cursor navigation mode");
	}
	if (
		request.pageLineCount !== undefined &&
		(
			!Number.isSafeInteger(request.pageLineCount) ||
			request.pageLineCount < 1
		)
	) {
		throw new RangeError("pageLineCount must be a positive safe integer");
	}
	if (
		request.preferredColumns &&
		(
			request.preferredColumns.length !== selections.selections.length ||
			request.preferredColumns.some(column =>
				!Number.isSafeInteger(column) || column < 1
			)
		)
	) {
		throw new RangeError("preferredColumns must match selections");
	}
	if (request.atomicTabSize !== undefined && (!Number.isSafeInteger(request.atomicTabSize) || request.atomicTabSize < 1)) {
		throw new RangeError('atomicTabSize must be a positive safe integer');
	}
	for (const selection of selections.selections) {
		model.offsetAt(selection.getSelectionStart());
		model.offsetAt(selection.getPosition());
	}
}

function isVerticalCommand(command: EditorCursorNavigationCommand): boolean {
	return command === EditorCursorNavigationCommand.LineUp ||
		command === EditorCursorNavigationCommand.LineDown ||
		command === EditorCursorNavigationCommand.PageUp ||
		command === EditorCursorNavigationCommand.PageDown;
}

function previousBoundary(boundaries: readonly number[], column: number): number {
	for (let index = boundaries.length - 1; index >= 0; index -= 1) {
		if (boundaries[index]! < column) return boundaries[index]!;
	}
	return 0;
}

function nextBoundary(boundaries: readonly number[], column: number): number {
	return boundaries.find(boundary => boundary > column) ??
		boundaries[boundaries.length - 1]!;
}

function boundaryAtOrBefore(boundaries: readonly number[], column: number): number {
	for (let index = boundaries.length - 1; index >= 0; index -= 1) {
		if (boundaries[index]! <= column) return boundaries[index]!;
	}
	return 0;
}

function selectionsEqual(left: Selection, right: Selection): boolean {
	return Position.compare(left.getSelectionStart(), right.getSelectionStart()) === 0 &&
		Position.compare(left.getPosition(), right.getPosition()) === 0;
}

function getTextWordRanges(text: string, wordPattern: RegExp | undefined): readonly { readonly start: number; readonly end: number }[] {
	if (!wordPattern) return getTextWordSegments(text).flatMap(segment => segment.wordLike ? [{ start: segment.start, end: segment.end }] : []);
	const flags = wordPattern.flags.replaceAll('y', '').includes('g') ? wordPattern.flags.replaceAll('y', '') : `${wordPattern.flags.replaceAll('y', '')}g`;
	const matcher = new RegExp(wordPattern.source, flags);
	const ranges: Array<{ readonly start: number; readonly end: number }> = [];
	for (let match = matcher.exec(text); match; match = matcher.exec(text)) {
		if (match[0].length === 0) {
			matcher.lastIndex += 1;
			continue;
		}
		ranges.push({ start: match.index, end: match.index + match[0].length });
	}
	return ranges;
}
