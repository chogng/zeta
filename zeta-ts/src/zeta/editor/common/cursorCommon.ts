import { CursorColumns } from './core/cursorColumns.js';
import { normalizeIndentation } from './core/misc/indentation.js';
import { Position } from './core/position.js';
import { Range } from './core/range.js';
import { type ISelection, Selection } from './core/selection.js';
import { type IEditorConfiguration } from './config/editorConfiguration.js';
import { EditorOption } from './config/editorOptions.js';
import { type TextModelResolvedOptions, PositionAffinity } from './model.js';
import { InputMode } from './inputMode.js';
import { AutoClosingPairs } from './languages/languageConfiguration.js';
import { type IComposableLanguageConfigurationService } from './languages/ownedLanguageConfigurationContributions.js';
import type { ICommand } from './editorCommon.js';

export interface IColumnSelectData {
	isReal: boolean;
	fromViewLineNumber: number;
	fromViewVisualColumn: number;
	toViewLineNumber: number;
	toViewVisualColumn: number;
}

export const enum EditOperationType {
	Other = 0,
	DeletingLeft = 2,
	DeletingRight = 3,
	TypingOther = 4,
	TypingFirstSpace = 5,
	TypingConsecutiveSpace = 6,
}

export interface CharacterMap {
	[char: string]: string;
}

const autoCloseAlways = (): boolean => true;
const autoCloseNever = (): boolean => false;
const autoCloseBeforeWhitespace = (character: string): boolean => character === ' ' || character === '\t';

/** Cursor policy resolved from the model, editor configuration, and language configuration. */
export class CursorConfiguration {
	_cursorMoveConfigurationBrand: void = undefined;

	public readonly readOnly: boolean;
	public readonly tabSize: number;
	public readonly indentSize: number;
	public readonly insertSpaces: boolean;
	public readonly stickyTabStops: boolean;
	public readonly pageSize: number;
	public readonly lineHeight: number;
	public readonly typicalHalfwidthCharacterWidth: number;
	public readonly useTabStops: boolean;
	public readonly trimWhitespaceOnDelete: boolean;
	public readonly wordSeparators: string;
	public readonly emptySelectionClipboard: boolean;
	public readonly copyWithSyntaxHighlighting: boolean;
	public readonly multiCursorMergeOverlapping: boolean;
	public readonly multiCursorPaste: 'spread' | 'full';
	public readonly multiCursorLimit: number;
	public readonly autoClosingBrackets;
	public readonly autoClosingComments;
	public readonly autoClosingQuotes;
	public readonly autoClosingDelete;
	public readonly autoClosingOvertype;
	public readonly autoSurround;
	public readonly autoIndent;
	public readonly autoClosingPairs: AutoClosingPairs;
	public readonly surroundingPairs: CharacterMap;
	public readonly blockCommentStartToken: string | null;
	public readonly shouldAutoCloseBefore: { quote: (character: string) => boolean; bracket: (character: string) => boolean; comment: (character: string) => boolean };
	public readonly wordSegmenterLocales: string[];
	public readonly overtypeOnPaste: boolean;

	private readonly languageId: string;

	constructor(
		languageId: string,
		modelOptions: TextModelResolvedOptions,
		configuration: IEditorConfiguration,
		public readonly languageConfigurationService: IComposableLanguageConfigurationService,
	) {
		this.languageId = languageId;
		const options = configuration.options;
		const layoutInfo = options.get(EditorOption.layoutInfo);
		const fontInfo = options.get(EditorOption.fontInfo);
		this.readOnly = options.get(EditorOption.readOnly);
		this.tabSize = modelOptions.tabSize;
		this.indentSize = modelOptions.indentSize;
		this.insertSpaces = modelOptions.insertSpaces;
		this.stickyTabStops = options.get(EditorOption.stickyTabStops);
		this.lineHeight = fontInfo.lineHeight;
		this.typicalHalfwidthCharacterWidth = fontInfo.typicalHalfwidthCharacterWidth;
		this.pageSize = Math.max(1, Math.floor(layoutInfo.height / this.lineHeight) - 2);
		this.useTabStops = options.get(EditorOption.useTabStops);
		this.trimWhitespaceOnDelete = options.get(EditorOption.trimWhitespaceOnDelete);
		this.wordSeparators = options.get(EditorOption.wordSeparators);
		this.emptySelectionClipboard = options.get(EditorOption.emptySelectionClipboard);
		this.copyWithSyntaxHighlighting = options.get(EditorOption.copyWithSyntaxHighlighting);
		this.multiCursorMergeOverlapping = options.get(EditorOption.multiCursorMergeOverlapping);
		this.multiCursorPaste = options.get(EditorOption.multiCursorPaste);
		this.multiCursorLimit = options.get(EditorOption.multiCursorLimit);
		this.autoClosingBrackets = options.get(EditorOption.autoClosingBrackets);
		this.autoClosingComments = options.get(EditorOption.autoClosingComments);
		this.autoClosingQuotes = options.get(EditorOption.autoClosingQuotes);
		this.autoClosingDelete = options.get(EditorOption.autoClosingDelete);
		this.autoClosingOvertype = options.get(EditorOption.autoClosingOvertype);
		this.autoSurround = options.get(EditorOption.autoSurround);
		this.autoIndent = options.get(EditorOption.autoIndent);
		this.wordSegmenterLocales = [...options.get(EditorOption.wordSegmenterLocales)];
		this.overtypeOnPaste = options.get(EditorOption.overtypeOnPaste);

		const language = languageConfigurationService.getLanguageConfiguration(languageId);
		this.autoClosingPairs = new AutoClosingPairs(language.autoClosingPairs);
		this.surroundingPairs = Object.fromEntries(language.surroundingPairs.map(pair => [pair.open, pair.close]));
		this.blockCommentStartToken = language.comments.blockComment?.open ?? null;
		this.shouldAutoCloseBefore = {
			quote: this.getShouldAutoClose(this.autoClosingQuotes, true),
			comment: this.getShouldAutoClose(this.autoClosingComments, false),
			bracket: this.getShouldAutoClose(this.autoClosingBrackets, false),
		};
	}

	public get electricChars(): Record<string, boolean> {
		return {};
	}

	public get inputMode(): 'insert' | 'overtype' {
		return InputMode.getInputMode();
	}

	public onElectricCharacter(_character: string, _context: unknown, _column: number): null {
		return null;
	}

	public normalizeIndentation(value: string): string {
		return normalizeIndentation(value, this.indentSize, this.insertSpaces);
	}

	public visibleColumnFromColumn(model: ICursorSimpleModel, position: Position): number {
		return CursorColumns.visibleColumnFromColumn(model.getLineContent(position.lineNumber), position.column, this.tabSize);
	}

	public columnFromVisibleColumn(model: ICursorSimpleModel, lineNumber: number, visibleColumn: number): number {
		const column = CursorColumns.columnFromVisibleColumn(model.getLineContent(lineNumber), visibleColumn, this.tabSize);
		return Math.min(model.getLineMaxColumn(lineNumber), Math.max(model.getLineMinColumn(lineNumber), column));
	}

	private getShouldAutoClose(strategy: typeof this.autoClosingQuotes, forQuotes: boolean): (character: string) => boolean {
		switch (strategy) {
			case 'always': return autoCloseAlways;
			case 'never': return autoCloseNever;
			case 'beforeWhitespace': return autoCloseBeforeWhitespace;
			case 'languageDefined': {
				const autoCloseBefore = this.languageConfigurationService.getLanguageConfiguration(this.languageId).autoCloseBefore;
				return character => autoCloseBefore.includes(character) || (forQuotes && character.length === 0);
			}
		}
	}
}

export interface ICursorSimpleModel {
	getLineCount(): number;
	getLineContent(lineNumber: number): string;
	getLineMinColumn(lineNumber: number): number;
	getLineMaxColumn(lineNumber: number): number;
	getLineFirstNonWhitespaceColumn(lineNumber: number): number;
	getLineLastNonWhitespaceColumn(lineNumber: number): number;
	normalizePosition(position: Position, affinity: PositionAffinity): Position;
	getLineIndentColumn(lineNumber: number): number;
}

export type PartialCursorState = CursorState | PartialModelCursorState | PartialViewCursorState;

export class CursorState {
	_cursorStateBrand: void = undefined;

	public static fromModelState(modelState: SingleCursorState): PartialModelCursorState {
		return new PartialModelCursorState(modelState);
	}

	public static fromViewState(viewState: SingleCursorState): PartialViewCursorState {
		return new PartialViewCursorState(viewState);
	}

	public static fromModelSelection(modelSelection: ISelection): PartialModelCursorState {
		const selection = Selection.liftSelection(modelSelection);
		return CursorState.fromModelState(new SingleCursorState(
			Range.fromPositions(selection.getSelectionStart()),
			SelectionStartKind.Simple,
			0,
			selection.getPosition(),
			0,
		));
	}

	public static fromModelSelections(modelSelections: readonly ISelection[]): PartialModelCursorState[] {
		return modelSelections.map(selection => this.fromModelSelection(selection));
	}

	constructor(
		public readonly modelState: SingleCursorState,
		public readonly viewState: SingleCursorState,
	) {}

	public equals(other: CursorState): boolean {
		return this.viewState.equals(other.viewState) && this.modelState.equals(other.modelState);
	}
}

export class PartialModelCursorState {
	public readonly viewState = null;

	constructor(public readonly modelState: SingleCursorState) {}
}

export class PartialViewCursorState {
	public readonly modelState = null;

	constructor(public readonly viewState: SingleCursorState) {}
}

export const enum SelectionStartKind {
	Simple,
	Word,
	Line,
}

export class SingleCursorState {
	_singleCursorStateBrand: void = undefined;

	public readonly selection: Selection;

	constructor(
		public readonly selectionStart: Range,
		public readonly selectionStartKind: SelectionStartKind,
		public readonly selectionStartLeftoverVisibleColumns: number,
		public readonly position: Position,
		public readonly leftoverVisibleColumns: number,
	) {
		this.selection = SingleCursorState._computeSelection(this.selectionStart, this.position);
	}

	public equals(other: SingleCursorState): boolean {
		return this.selectionStartLeftoverVisibleColumns === other.selectionStartLeftoverVisibleColumns
			&& this.leftoverVisibleColumns === other.leftoverVisibleColumns
			&& this.selectionStartKind === other.selectionStartKind
			&& this.position.equals(other.position)
			&& this.selectionStart.equalsRange(other.selectionStart);
	}

	public hasSelection(): boolean {
		return !this.selection.isEmpty() || !this.selectionStart.isEmpty();
	}

	public move(inSelectionMode: boolean, lineNumber: number, column: number, leftoverVisibleColumns: number): SingleCursorState {
		if (inSelectionMode) {
			return new SingleCursorState(this.selectionStart, this.selectionStartKind, this.selectionStartLeftoverVisibleColumns, new Position(lineNumber, column), leftoverVisibleColumns);
		}
		return new SingleCursorState(new Range(lineNumber, column, lineNumber, column), SelectionStartKind.Simple, leftoverVisibleColumns, new Position(lineNumber, column), leftoverVisibleColumns);
	}

	private static _computeSelection(selectionStart: Range, position: Position): Selection {
		return selectionStart.isEmpty() || !position.isBeforeOrEqual(selectionStart.getStartPosition())
			? Selection.fromPositions(selectionStart.getStartPosition(), position)
			: Selection.fromPositions(selectionStart.getEndPosition(), position);
	}
}

export class EditOperationResult {
	_editOperationResultBrand: void = undefined;

	readonly type: EditOperationType;
	readonly commands: Array<ICommand | null>;
	readonly shouldPushStackElementBefore: boolean;
	readonly shouldPushStackElementAfter: boolean;

	constructor(type: EditOperationType, commands: Array<ICommand | null>, options: { shouldPushStackElementBefore: boolean; shouldPushStackElementAfter: boolean }) {
		this.type = type;
		this.commands = commands;
		this.shouldPushStackElementBefore = options.shouldPushStackElementBefore;
		this.shouldPushStackElementAfter = options.shouldPushStackElementAfter;
	}
}

export function isQuote(character: string): boolean {
	return character === "'" || character === '"' || character === '`';
}
