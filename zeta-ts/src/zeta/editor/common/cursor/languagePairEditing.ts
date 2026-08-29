import { DeleteOperations } from './cursorDeleteOperations.js';
import { TypeWithoutInterceptorsOperation, type SelectionEdit } from './cursorTypeEditOperations.js';
import { EditorCommandHistoryMode, type EditorEditCommand } from "../commands/editorEditCommand.js";
import { type LanguageAutoClosingPair, type LanguageCharacterPair, type ResolvedLanguageConfiguration } from "../languages/languageConfiguration.js";
import { type LanguageLexicalContextSource } from "../languages/languageLexicalContext.js";
import { type Selection } from "../core/selection.js";
import type { SelectionSet } from "./selectionSet.js";
import { Position } from "../core/position.js";
import { Range } from "../core/range.js";
import { type TextModel } from "../model/textModel.js";

export interface LanguagePairTypeCommand {
	readonly command: EditorEditCommand;
	readonly didInsertText: boolean;
	readonly autoClosingActions: readonly LanguageAutoClosingAction[];
}

export interface LanguageAutoClosingAction {
	readonly open: string;
	readonly close: string;
	readonly enclosingStartOffset: number;
	readonly closeStartOffset: number;
	readonly closeEndOffset: number;
}

export interface LanguageAutoClosingTrust {
	canOvertype(position: Position, close: string): boolean;
	canDeletePair(position: Position, pair: LanguageCharacterPair): boolean;
}

export interface LanguagePairTypeOptions {
	readonly autoClosingTrust?: LanguageAutoClosingTrust;
	readonly lexicalContext?: LanguageLexicalContextSource;
}

interface PairTypeEdit {
	readonly edit: SelectionEdit;
	readonly didInsertText: boolean;
	readonly autoClosingPair?: LanguageAutoClosingPair;
}

/** Creates language-aware surround, auto-close, or closing-token overtype. */
export function createLanguagePairTypeCommand(model: TextModel, selections: SelectionSet, text: string, configuration: ResolvedLanguageConfiguration, options: LanguagePairTypeOptions = {}): LanguagePairTypeCommand | undefined {
	if (typeof text !== "string") throw new TypeError("Language pair input text must be a string");
	assertConfiguration(configuration);
	assertOptions(model, configuration, options);
	const surroundingPair = configuration.surroundingPairs.find(pair => pair.open === text);
	const autoClosingPair = configuration.autoClosingPairs.find(pair => pair.open === text);
	const closingPairs = configuration.autoClosingPairs.filter(pair => pair.close === text);
	if (!surroundingPair && !autoClosingPair && closingPairs.length === 0) return undefined;
	const pairEdits = selections.selections.map(selection => createPairTypeEdit(model, selection, text, configuration, surroundingPair, autoClosingPair, closingPairs, options));
	const command = TypeWithoutInterceptorsOperation.getEdits(model, selections, pairEdits.map(result => result.edit), EditorCommandHistoryMode.CoalesceTyping);
	return Object.freeze({
		command,
		didInsertText: pairEdits.some(result => result.didInsertText),
		autoClosingActions: createAutoClosingActions(model, pairEdits),
	});
}

/** Deletes both sides of an empty configured pair, falling back per selection. */
export function createLanguagePairBackspaceCommand(model: TextModel, selections: SelectionSet, configuration: ResolvedLanguageConfiguration, trust?: LanguageAutoClosingTrust): EditorEditCommand | undefined {
	assertConfiguration(configuration);
	let paired = false;
	const edits = selections.selections.map(selection => {
		if (!selection.isEmpty()) return collapsedEdit(selection);
		const pairRange = getEmptyPairRange(model, selection.getPosition(), configuration.autoClosingPairs, trust);
		if (pairRange) {
			paired = true;
			return collapsedEdit(pairRange);
		}
		return collapsedEdit(DeleteOperations.getPreviousDeleteRange(model, selection.getPosition()));
	});
	if (!paired) return undefined;
	return TypeWithoutInterceptorsOperation.getEdits(model, selections, edits, EditorCommandHistoryMode.CoalesceBackspace);
}

function createPairTypeEdit(model: TextModel, selection: Selection, text: string, configuration: ResolvedLanguageConfiguration, surroundingPair: LanguageCharacterPair | undefined, autoClosingPair: LanguageAutoClosingPair | undefined, closingPairs: readonly LanguageAutoClosingPair[], options: LanguagePairTypeOptions): PairTypeEdit {
	if (!selection.isEmpty() && surroundingPair) {
		const selectedText = model.getTextInRange(selection);
		const replacement = surroundingPair.open + selectedText + surroundingPair.close;
		const start = surroundingPair.open.length;
		const end = start + selectedText.length;
		const forward = Position.compare(selection.getSelectionStart(), selection.getStartPosition()) === 0;
		return {
			edit: {
				range: selection,
				text: replacement,
				anchorOffsetInText: forward ? start : end,
				activeOffsetInText: forward ? end : start,
			},
			didInsertText: true,
		};
	}
	if (selection.isEmpty()) {
		const line = model.getLineContent(selection.getPosition().lineNumber);
		const columnIndex = selection.getPosition().column - 1;
		const closingPair = closingPairs.find(pair => line.startsWith(pair.close, columnIndex) && options.autoClosingTrust?.canOvertype(selection.getPosition(), pair.close) === true);
		if (closingPair) {
			return {
				edit: {
					range: selection,
					text: "",
					anchorOffsetInText: closingPair.close.length,
					activeOffsetInText: closingPair.close.length,
				},
				didInsertText: false,
			};
		}
		if (autoClosingPair && shouldAutoClose(line, columnIndex, configuration.autoCloseBefore) && isAutoClosingAllowed(selection.getPosition(), autoClosingPair, options.lexicalContext)) {
			return {
				edit: {
					range: selection,
					text: autoClosingPair.open + autoClosingPair.close,
					anchorOffsetInText: autoClosingPair.open.length,
					activeOffsetInText: autoClosingPair.open.length,
				},
				didInsertText: true,
				autoClosingPair,
			};
		}
	}
	return {
		edit: {
			range: selection,
			text,
			anchorOffsetInText: text.length,
			activeOffsetInText: text.length,
		},
		didInsertText: true,
	};
}

function getEmptyPairRange(model: TextModel, position: Position, pairs: readonly LanguageCharacterPair[], trust: LanguageAutoClosingTrust | undefined): Range | undefined {
	const line = model.getLineContent(position.lineNumber);
	const columnIndex = position.column - 1;
	const pair = [...pairs].sort((left, right) => right.open.length - left.open.length).find(candidate => (
		trust?.canDeletePair(position, candidate) === true &&
		columnIndex >= candidate.open.length &&
		line.slice(columnIndex - candidate.open.length, columnIndex) === candidate.open &&
		line.startsWith(candidate.close, columnIndex)
	));
	if (!pair) return undefined;
	return Range.fromPositions(
		new Position(position.lineNumber, position.column - pair.open.length),
		new Position(position.lineNumber, position.column + pair.close.length),
	);
}

function createAutoClosingActions(model: TextModel, pairEdits: readonly PairTypeEdit[]): readonly LanguageAutoClosingAction[] {
	const ordered = pairEdits.map((pairEdit, selectionIndex) => ({
		pairEdit,
		selectionIndex,
		startOffset: model.offsetAt(pairEdit.edit.range.getStartPosition()),
		endOffset: model.offsetAt(pairEdit.edit.range.getEndPosition()),
	})).sort((left, right) => left.startOffset - right.startOffset || left.endOffset - right.endOffset || left.selectionIndex - right.selectionIndex);
	const actions: LanguageAutoClosingAction[] = [];
	let cumulativeDelta = 0;
	for (const item of ordered) {
		const pair = item.pairEdit.autoClosingPair;
		if (pair) {
			const enclosingStartOffset = item.startOffset + cumulativeDelta;
			const closeStartOffset = enclosingStartOffset + pair.open.length;
			actions.push(Object.freeze({
				open: pair.open,
				close: pair.close,
				enclosingStartOffset,
				closeStartOffset,
				closeEndOffset: closeStartOffset + pair.close.length,
			}));
		}
		cumulativeDelta += item.pairEdit.edit.text.length - (item.endOffset - item.startOffset);
	}
	return Object.freeze(actions);
}

function collapsedEdit(range: Range): SelectionEdit {
	return {
		range,
		text: "",
		anchorOffsetInText: 0,
		activeOffsetInText: 0,
	};
}

function shouldAutoClose(line: string, column: number, autoCloseBefore: string): boolean {
	if (column >= line.length) return true;
	const next = String.fromCodePoint(line.codePointAt(column)!);
	return autoCloseBefore.includes(next);
}

function assertConfiguration(configuration: ResolvedLanguageConfiguration): void {
	if (typeof configuration !== "object" || configuration === null || !Array.isArray(configuration.autoClosingPairs) || !Array.isArray(configuration.surroundingPairs) || typeof configuration.autoCloseBefore !== "string") {
		throw new TypeError("Language pair editing requires a resolved language configuration");
	}
}

function isAutoClosingAllowed(position: Position, pair: LanguageAutoClosingPair, lexicalContext: LanguageLexicalContextSource | undefined): boolean {
	if (!pair.notIn || pair.notIn.length === 0 || !lexicalContext) return true;
	const tokenType = lexicalContext.getTokenTypeAt(position);
	return tokenType !== "string" && tokenType !== "comment" || !pair.notIn.includes(tokenType);
}

function assertOptions(model: TextModel, configuration: ResolvedLanguageConfiguration, options: LanguagePairTypeOptions): void {
	if (typeof options !== "object" || options === null) throw new TypeError("Language pair editing options must be an object");
	const lexicalContext = options.lexicalContext;
	if (lexicalContext && (
		lexicalContext.textModel !== model ||
		!lexicalContext.supportsLanguageId(configuration.languageId) ||
		typeof lexicalContext.getTokenTypeAt !== "function"
	)) {
		throw new TypeError("Language pair lexical context must match its model and language");
	}
}
