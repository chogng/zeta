import { clamp } from "../../../base/common/numbers.js";
import { TextSelection, TextSelectionSet } from "../core/selection.js";
import { TextPosition } from "../core/text.js";
import { type TextModel } from "../model/textModel.js";
import { getTextGraphemeBoundaries } from "../core/textSegmentation.js";
import { getTextWordRanges } from "./wordBoundary.js";

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
}

export interface EditorCursorNavigationResult {
	readonly selections: TextSelectionSet;
	readonly preferredColumns: readonly number[] | undefined;
}

/**
 * Applies one DOM-independent cursor navigation command to every selection.
 *
 * Vertical commands retain caller-owned preferred UTF-16 columns. Exact
 * duplicate results coalesce while preserving the primary selection mapping.
 */
export function navigateEditorCursors(model: TextModel, selections: TextSelectionSet, request: EditorCursorNavigationRequest): EditorCursorNavigationResult {
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
		);
		return request.mode === EditorCursorNavigationMode.Extend
			? TextSelection.from(selection.anchor, target)
			: TextSelection.collapsedAt(target);
	});
	return normalizeResult(
		navigated,
		selections.primaryIndex,
		preferredColumns,
	);
}

function navigationTarget(
	model: TextModel,
	selection: TextSelection,
	command: EditorCursorNavigationCommand,
	pageLineCount: number,
	preferredColumn: number | undefined,
	mode: EditorCursorNavigationMode,
	wordPattern: RegExp | undefined,
): TextPosition {
	if (
		mode === EditorCursorNavigationMode.Move &&
		!selection.collapsed
	) {
		if (
			command === EditorCursorNavigationCommand.CharacterLeft ||
			command === EditorCursorNavigationCommand.WordLeft
		) {
			return selection.range.start;
		}
		if (
			command === EditorCursorNavigationCommand.CharacterRight ||
			command === EditorCursorNavigationCommand.WordRight
		) {
			return selection.range.end;
		}
	}

	const active = selection.active;
	switch (command) {
		case EditorCursorNavigationCommand.CharacterLeft:
			return previousCharacter(model, active);
		case EditorCursorNavigationCommand.CharacterRight:
			return nextCharacter(model, active);
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
			return TextPosition.at(active.lineIndex, 0);
		case EditorCursorNavigationCommand.LineEnd:
			return TextPosition.at(
				active.lineIndex,
				model.getLineContent(active.lineIndex).length,
			);
		case EditorCursorNavigationCommand.DocumentStart:
			return TextPosition.at(0, 0);
		case EditorCursorNavigationCommand.DocumentEnd: {
			const lineIndex = model.lineCount - 1;
			return TextPosition.at(
				lineIndex,
				model.getLineContent(lineIndex).length,
			);
		}
	}
}

function previousCharacter(model: TextModel, position: TextPosition): TextPosition {
	if (position.columnIndex === 0) {
		if (position.lineIndex === 0) return position;
		const lineIndex = position.lineIndex - 1;
		return TextPosition.at(
			lineIndex,
			model.getLineContent(lineIndex).length,
		);
	}
	const boundaries = getTextGraphemeBoundaries(
		model.getLineContent(position.lineIndex),
	);
	return TextPosition.at(
		position.lineIndex,
		previousBoundary(boundaries, position.columnIndex),
	);
}

function nextCharacter(model: TextModel, position: TextPosition): TextPosition {
	const line = model.getLineContent(position.lineIndex);
	if (position.columnIndex === line.length) {
		return position.lineIndex + 1 < model.lineCount
			? TextPosition.at(position.lineIndex + 1, 0)
			: position;
	}
	return TextPosition.at(
		position.lineIndex,
		nextBoundary(getTextGraphemeBoundaries(line), position.columnIndex),
	);
}

function previousWord(model: TextModel, position: TextPosition, wordPattern: RegExp | undefined): TextPosition {
	for (let lineIndex = position.lineIndex; lineIndex >= 0; lineIndex -= 1) {
		const limit = lineIndex === position.lineIndex
			? position.columnIndex
			: Number.POSITIVE_INFINITY;
		const segments = getTextWordRanges(model.getLineContent(lineIndex), wordPattern);
		for (let index = segments.length - 1; index >= 0; index -= 1) {
			const segment = segments[index]!;
			if (segment.start < limit) {
				return TextPosition.at(lineIndex, segment.start);
			}
		}
	}
	return TextPosition.at(0, 0);
}

function nextWord(model: TextModel, position: TextPosition, wordPattern: RegExp | undefined): TextPosition {
	for (
		let lineIndex = position.lineIndex;
		lineIndex < model.lineCount;
		lineIndex += 1
	) {
		const limit = lineIndex === position.lineIndex
			? position.columnIndex
			: -1;
		for (const segment of getTextWordRanges(model.getLineContent(lineIndex), wordPattern)) {
			if (segment.start > limit) {
				return TextPosition.at(lineIndex, segment.start);
			}
		}
	}
	const lineIndex = model.lineCount - 1;
	return TextPosition.at(
		lineIndex,
		model.getLineContent(lineIndex).length,
	);
}

function verticalTarget(model: TextModel, position: TextPosition, lineDelta: number, preferredColumn: number | undefined): TextPosition {
	const lineIndex = clamp(
		position.lineIndex + lineDelta,
		0,
		model.lineCount - 1,
	);
	if (lineIndex === position.lineIndex) return position;
	const line = model.getLineContent(lineIndex);
	const column = Math.min(preferredColumn ?? position.columnIndex, line.length);
	return TextPosition.at(
		lineIndex,
		boundaryAtOrBefore(getTextGraphemeBoundaries(line), column),
	);
}

function resolvePreferredColumns(selections: TextSelectionSet, preferredColumns: readonly number[] | undefined): readonly number[] {
	if (preferredColumns?.length === selections.selections.length) {
		return Object.freeze([...preferredColumns]);
	}
	return Object.freeze(
		selections.selections.map(selection => selection.active.columnIndex),
	);
}

function normalizeResult(selections: readonly TextSelection[], primaryIndex: number, preferredColumns: readonly number[] | undefined): EditorCursorNavigationResult {
	const normalized: TextSelection[] = [];
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
		selections: TextSelectionSet.withPrimary(
			normalized,
			sourceToNormalized[primaryIndex]!,
		),
		preferredColumns: preferredColumns
			? Object.freeze(normalizedColumns)
			: undefined,
	});
}

function validateRequest(model: TextModel, selections: TextSelectionSet, request: EditorCursorNavigationRequest): void {
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
				!Number.isSafeInteger(column) || column < 0
			)
		)
	) {
		throw new RangeError("preferredColumns must match selections");
	}
	for (const selection of selections.selections) {
		model.offsetAt(selection.anchor);
		model.offsetAt(selection.active);
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

function selectionsEqual(left: TextSelection, right: TextSelection): boolean {
	return left.anchor.compareTo(right.anchor) === 0 &&
		left.active.compareTo(right.active) === 0;
}
