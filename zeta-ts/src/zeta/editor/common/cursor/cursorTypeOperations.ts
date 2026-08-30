import { getOrSet } from '../../../base/common/map.js';
import { EditorCommandHistoryMode, type EditorEditCommand, type TextSelectionOffsets } from '../commands/editorEditCommand.js';
import { Position } from '../core/position.js';
import { Range } from '../core/range.js';
import { normalizeTextLineEndings } from '../core/textChange.js';
import { getEditorIndentationUnit, getLeadingIndentation, normalizeEditorIndentation, normalizeEditorIndentationText, resolveEditorIndentationOptions, unshiftEditorIndentation, type EditorIndentationOptions, type ResolvedEditorIndentationOptions } from '../core/misc/indentation.js';

import { type TextModel } from '../model/textModel.js';
import { TypeWithoutInterceptorsOperation, type SelectionEdit } from './cursorTypeEditOperations.js';
import { type TextEdit } from '../languages.js';
import { type Selection } from '../core/selection.js';
import { IndentAction, StandardAutoClosingPairConditional, type EnterAction, type IAutoClosingPair, type OnEnterRule } from '../languages/languageConfiguration.js';
import { type ResolvedLanguageConfiguration } from '../languages/languageConfigurationRegistry.js';
import { type LanguageLexicalContextSource } from '../languages/languageLexicalContext.js';
import { StandardTokenType } from '../encodedTokenAttributes.js';

interface PairTypeEdit {
	readonly edit: SelectionEdit;
	readonly insertedText: boolean;
	readonly pair?: StandardAutoClosingPairConditional;
}

interface OffsetRange {
	readonly startOffset: number;
	readonly endOffset: number;
}

interface AutoClosedRanges {
	readonly characters: readonly OffsetRange[];
	readonly enclosing: readonly OffsetRange[];
}

export class TypeOperations {
	/** Applies language-aware surrounding, auto-closing, and closing-token overtype. */
	public static typeWithInterceptors(
		model: TextModel,
		selections: readonly Selection[],
		text: string,
		configuration: ResolvedLanguageConfiguration,
		autoClosedCharacters: readonly Range[] = [],
		lexicalContext?: LanguageLexicalContextSource,
	): {
		readonly command: EditorEditCommand;
		readonly insertedText: boolean;
		readonly autoClosedCharacters: readonly OffsetRange[];
		readonly autoClosedEnclosing: readonly OffsetRange[];
	} | undefined {
		if (typeof text !== 'string') throw new TypeError('Typed text must be a string');
		assertConfiguration(configuration);
		assertLexicalContext(model, configuration, lexicalContext);
		const surroundingPair = configuration.getSurroundingPairs().find(pair => pair.open === text);
		const pairs = configuration.characterPair.getAutoClosingPairs();
		const openingPair = pairs.find(pair => pair.open === text);
		const closingPairs = pairs.filter(pair => pair.close === text);
		if (!surroundingPair && !openingPair && closingPairs.length === 0) return undefined;
		const autoCloseBefore = configuration.getAutoCloseBeforeSet(text === "'" || text === '"' || text === '`');
		const edits = selections.map(selection => pairTypeEdit(
			model,
			selection,
			text,
			surroundingPair,
			openingPair,
			closingPairs,
			autoCloseBefore,
			autoClosedCharacters,
			lexicalContext,
		));
		const ranges = autoClosingRanges(model, edits);
		return Object.freeze({
			command: TypeWithoutInterceptorsOperation.getEdits(
				model,
				selections,
				edits.map(result => result.edit),
				EditorCommandHistoryMode.CoalesceTyping,
			),
			insertedText: edits.some(result => result.insertedText),
			autoClosedCharacters: ranges.characters,
			autoClosedEnclosing: ranges.enclosing,
		});
	}

	/** Creates one language-aware Enter transaction for all current selections. */
	public static enter(
		model: TextModel,
		selections: readonly Selection[],
		configuration: ResolvedLanguageConfiguration,
		indentation?: EditorIndentationOptions,
		lexicalContext?: LanguageLexicalContextSource,
	): EditorEditCommand {
		assertConfiguration(configuration);
		assertLexicalContext(model, configuration, lexicalContext);
		const resolvedIndentation = resolveEditorIndentationOptions(indentation);
		const edits = selections.map(selection => enterEdit(model, selection, configuration, resolvedIndentation, lexicalContext));
		return TypeWithoutInterceptorsOperation.getEdits(model, selections, edits, EditorCommandHistoryMode.BeginCoalescedTyping);
	}

	public static typeWithoutInterceptors(model: TextModel, selections: readonly Selection[], text: string): EditorEditCommand {
		if (typeof text !== 'string') throw new TypeError('Typed text must be a string');
		const normalized = normalizeTextLineEndings(text);
		return TypeWithoutInterceptorsOperation.getEdits(
			model,
			selections,
			selections.map(selection => textEdit(selection, normalized, normalized.length)),
			EditorCommandHistoryMode.CoalesceTyping,
		);
	}

	public static paste(model: TextModel, selections: readonly Selection[], text: string): EditorEditCommand {
		if (typeof text !== 'string') throw new TypeError('Pasted text must be a string');
		const normalized = normalizeTextLineEndings(text);
		return TypeWithoutInterceptorsOperation.getEdits(
			model,
			selections,
			selections.map(selection => textEdit(selection, normalized, normalized.length)),
			EditorCommandHistoryMode.Isolated,
		);
	}

	public static distributedPaste(model: TextModel, selections: readonly Selection[], texts: readonly string[]): EditorEditCommand {
		if (texts.length !== selections.length) throw new RangeError('Distributed paste text must match the selection count');
		const orderedSelections = [...selections].sort(Range.compareRangesUsingStarts);
		const normalized = texts.map(text => {
			if (typeof text !== 'string') throw new TypeError('Distributed paste text must contain only strings');
			return normalizeTextLineEndings(text);
		});
		return TypeWithoutInterceptorsOperation.getEdits(
			model,
			orderedSelections,
			orderedSelections.map((selection, selectionIndex) => textEdit(
				selection,
				normalized[selectionIndex]!,
				normalized[selectionIndex]!.length,
			)),
			EditorCommandHistoryMode.Isolated,
		);
	}

	public static linePaste(model: TextModel, selections: readonly Selection[], texts: readonly string[]): EditorEditCommand {
		if (texts.length !== selections.length) throw new RangeError('Line paste text must match the selection count');
		const orderedSelections = [...selections].sort(Range.compareRangesUsingStarts);
		const normalized = texts.map(text => {
			if (typeof text !== 'string') throw new TypeError('Line paste text must contain only strings');
			const value = normalizeTextLineEndings(text);
			if (!value.endsWith('\n')) throw new RangeError('Line paste text must end with a line break');
			return value;
		});
		const groups = new Map<number, { readonly lineNumber: number; readonly selectionIndices: number[]; text: string }>();
		for (let selectionIndex = 0; selectionIndex < orderedSelections.length; selectionIndex += 1) {
			const selection = orderedSelections[selectionIndex]!;
			if (!selection.isEmpty()) throw new RangeError('Line paste requires collapsed selections');
			const lineNumber = selection.getPosition().lineNumber;
			const group = getOrSet(groups, lineNumber, { lineNumber, selectionIndices: [], text: '' });
			group.selectionIndices.push(selectionIndex);
			group.text += normalized[selectionIndex]!;
		}
		const sorted = [...groups.values()].sort((left, right) => left.lineNumber - right.lineNumber);
		const selectionsAfter = new Array<TextSelectionOffsets>(orderedSelections.length);
		const edits: TextEdit[] = [];
		let cumulativeDelta = 0;
		for (const group of sorted) {
			const position = new Position(group.lineNumber, 1);
			const startOffset = model.offsetAt(position);
			edits.push({ range: Range.fromPositions(position), text: group.text });
			for (const selectionIndex of group.selectionIndices) {
				const columnIndex = orderedSelections[selectionIndex]!.getPosition().column - 1;
				const caretOffset = startOffset + cumulativeDelta + group.text.length + columnIndex;
				selectionsAfter[selectionIndex] = { anchorOffset: caretOffset, activeOffset: caretOffset };
			}
			cumulativeDelta += group.text.length;
		}
		const normalizedSelections = TypeWithoutInterceptorsOperation.normalizeSelectionOffsets(selectionsAfter, 0);
		return {
			edits: Object.freeze(edits),
			selectionsAfter: normalizedSelections.selections,
			primarySelectionIndex: normalizedSelections.primaryIndex,
			historyMode: EditorCommandHistoryMode.Isolated,
		};
	}
}

function pairTypeEdit(
	model: TextModel,
	selection: Selection,
	text: string,
	surroundingPair: IAutoClosingPair | undefined,
	openingPair: StandardAutoClosingPairConditional | undefined,
	closingPairs: readonly StandardAutoClosingPairConditional[],
	autoCloseBefore: string,
	autoClosedCharacters: readonly Range[],
	lexicalContext: LanguageLexicalContextSource | undefined,
): PairTypeEdit {
	if (!selection.isEmpty() && surroundingPair) {
		const selectedText = model.getTextInRange(selection);
		const replacement = surroundingPair.open + selectedText + surroundingPair.close;
		const start = surroundingPair.open.length;
		const end = start + selectedText.length;
		const forward = Position.compare(selection.getSelectionStart(), selection.getStartPosition()) === 0;
		return {
			edit: { range: selection, text: replacement, anchorOffsetInText: forward ? start : end, activeOffsetInText: forward ? end : start },
			insertedText: true,
		};
	}
	if (selection.isEmpty()) {
		const position = selection.getPosition();
		const line = model.getLineContent(position.lineNumber);
		const columnIndex = position.column - 1;
		const closingPair = closingPairs.find(pair => line.startsWith(pair.close, columnIndex) && ownsCloser(model, position, pair.close, autoClosedCharacters));
		if (closingPair) {
			return {
				edit: { range: selection, text: '', anchorOffsetInText: closingPair.close.length, activeOffsetInText: closingPair.close.length },
				insertedText: false,
			};
		}
		if (openingPair && shouldAutoClose(line, columnIndex, autoCloseBefore) && autoClosingAllowed(position, openingPair, lexicalContext)) {
			return {
				edit: { range: selection, text: openingPair.open + openingPair.close, anchorOffsetInText: openingPair.open.length, activeOffsetInText: openingPair.open.length },
				insertedText: true,
				pair: openingPair,
			};
		}
	}
	return {
		edit: { range: selection, text, anchorOffsetInText: text.length, activeOffsetInText: text.length },
		insertedText: true,
	};
}

function autoClosingRanges(model: TextModel, edits: readonly PairTypeEdit[]): AutoClosedRanges {
	const ordered = edits.map((result, index) => ({
		result,
		index,
		startOffset: model.offsetAt(result.edit.range.getStartPosition()),
		endOffset: model.offsetAt(result.edit.range.getEndPosition()),
	})).sort((left, right) => left.startOffset - right.startOffset || left.endOffset - right.endOffset || left.index - right.index);
	const characters: OffsetRange[] = [];
	const enclosing: OffsetRange[] = [];
	let delta = 0;
	for (const item of ordered) {
		const pair = item.result.pair;
		if (pair) {
			const start = item.startOffset + delta;
			const closeStart = start + pair.open.length;
			const end = closeStart + pair.close.length;
			characters.push(Object.freeze({ startOffset: closeStart, endOffset: end }));
			enclosing.push(Object.freeze({ startOffset: start, endOffset: end }));
		}
		delta += item.result.edit.text.length - (item.endOffset - item.startOffset);
	}
	return { characters: Object.freeze(characters), enclosing: Object.freeze(enclosing) };
}

function ownsCloser(model: TextModel, position: Position, close: string, ranges: readonly Range[]): boolean {
	return ranges.some(range => Position.equals(range.getStartPosition(), position) && model.getTextInRange(range) === close);
}

function shouldAutoClose(line: string, columnIndex: number, autoCloseBefore: string): boolean {
	if (columnIndex >= line.length) return true;
	return autoCloseBefore.includes(String.fromCodePoint(line.codePointAt(columnIndex)!));
}

function autoClosingAllowed(position: Position, pair: StandardAutoClosingPairConditional, lexicalContext: LanguageLexicalContextSource | undefined): boolean {
	if (!lexicalContext) return true;
	const tokenType = lexicalContext.getTokenTypeAt(position) ?? (position.column > 1
		? lexicalContext.getTokenTypeAt(new Position(position.lineNumber, position.column - 1))
		: undefined);
	return pair.isOK(tokenType === 'string'
		? StandardTokenType.String
		: tokenType === 'comment'
			? StandardTokenType.Comment
			: tokenType === 'regexp'
				? StandardTokenType.RegEx
				: StandardTokenType.Other);
}

function enterEdit(model: TextModel, selection: Selection, configuration: ResolvedLanguageConfiguration, indentation: ResolvedEditorIndentationOptions, lexicalContext: LanguageLexicalContextSource | undefined): SelectionEdit {
	const startLine = model.getLineContent(selection.getStartPosition().lineNumber);
	const endLine = model.getLineContent(selection.getEndPosition().lineNumber);
	const originalBefore = startLine.slice(0, selection.startColumn - 1);
	const before = lexicalContext?.getStructuralLineContent(selection.startLineNumber - 1, 0, selection.startColumn - 1) ?? originalBefore;
	const after = lexicalContext?.getStructuralLineContent(selection.endLineNumber - 1, selection.endColumn - 1, endLine.length) ?? endLine.slice(selection.endColumn - 1);
	const previous = selection.startLineNumber > 1
		? lexicalContext?.getStructuralLineContent(selection.startLineNumber - 2) ?? model.getLineContent(selection.startLineNumber - 1)
		: '';
	const insertion = enterInsertion(originalBefore, enterAction(configuration, previous, before, after), indentation);
	return { range: selection, text: insertion.text, anchorOffsetInText: insertion.caret, activeOffsetInText: insertion.caret };
}

function enterAction(configuration: ResolvedLanguageConfiguration, previous: string, before: string, after: string): EnterAction {
	const explicit = configuration.underlyingConfig.onEnterRules?.find(rule => matchesEnterRule(rule, previous, before, after));
	if (explicit) return explicit.action;
	const pairs = [...(configuration.underlyingConfig.brackets ?? [])].sort((left, right) => right[0].length - left[0].length);
	for (const pair of pairs) {
		if (!before.trimEnd().endsWith(pair[0])) continue;
		return after.trimStart().startsWith(pair[1])
			? { indentAction: IndentAction.IndentOutdent }
			: { indentAction: IndentAction.Indent };
	}
	const rules = configuration.indentationRules;
	if (rules && !testPattern(rules.unIndentedLinePattern, before)) {
		if (testPattern(rules.increaseIndentPattern, before) || testPattern(rules.indentNextLinePattern, before)) return { indentAction: IndentAction.Indent };
		if (testPattern(rules.decreaseIndentPattern, after)) return { indentAction: IndentAction.Outdent };
	}
	return { indentAction: IndentAction.None };
}

function enterInsertion(before: string, action: EnterAction, indentation: ResolvedEditorIndentationOptions): { readonly text: string; readonly caret: number } {
	const leading = getLeadingIndentation(before);
	const removeText = Math.min(action.removeText ?? 0, leading.length);
	const base = normalizeEditorIndentation(leading.slice(0, leading.length - removeText), indentation);
	const unit = getEditorIndentationUnit(indentation);
	if (action.indentAction === IndentAction.IndentOutdent) {
		const first = normalizeEditorIndentationText(base + (action.appendText ?? unit), indentation);
		return { text: '\n' + first + '\n' + base, caret: 1 + first.length };
	}
	const target = action.indentAction === IndentAction.Outdent
		? unshiftEditorIndentation(base, indentation) + (action.appendText ?? '')
		: base + (action.indentAction === IndentAction.Indent ? unit + (action.appendText ?? '') : action.appendText ?? '');
	const normalized = normalizeEditorIndentationText(target, indentation);
	return { text: '\n' + normalized, caret: 1 + normalized.length };
}

function matchesEnterRule(rule: OnEnterRule, previous: string, before: string, after: string): boolean {
	return testPattern(rule.beforeText, before)
		&& (rule.afterText === undefined || testPattern(rule.afterText, after))
		&& (rule.previousLineText === undefined || testPattern(rule.previousLineText, previous));
}

function testPattern(pattern: RegExp | null | undefined, text: string): boolean {
	return pattern ? new RegExp(pattern.source, pattern.flags).test(text) : false;
}

function assertConfiguration(configuration: ResolvedLanguageConfiguration): void {
	if (typeof configuration !== 'object' || configuration === null || typeof configuration.getAutoClosingPairs !== 'function') {
		throw new TypeError('Language editing requires a resolved language configuration');
	}
}

function assertLexicalContext(model: TextModel, configuration: ResolvedLanguageConfiguration, lexicalContext: LanguageLexicalContextSource | undefined): void {
	if (lexicalContext && (
		lexicalContext.textModel !== model
		|| !lexicalContext.supportsLanguageId(configuration.languageId)
		|| typeof lexicalContext.getTokenTypeAt !== 'function'
	)) {
		throw new TypeError('Language editing lexical context must match its model and language');
	}
}

function textEdit(range: Range, text: string, caretOffsetInText: number): SelectionEdit {
	return { range, text, anchorOffsetInText: caretOffsetInText, activeOffsetInText: caretOffsetInText };
}
