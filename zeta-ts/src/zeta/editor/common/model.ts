/** Selects one visual side when a model position has multiple rendered locations. */
export enum PositionAffinity {
	Left = 0,
	Right = 1,
	None = 2,
	LeftOfInjectedText = 3,
	RightOfInjectedText = 4,
}

/** Text direction for a decoration. */
export enum TextDirection {
	LTR = 0,
	RTL = 1,
}

/** Vertical lane in the glyph margin. */
export enum GlyphMarginLane {
	Left = 1,
	Center = 2,
	Right = 3,
}

/** Describes how a tracked range grows when typing at its edges. */
export enum TrackedRangeStickiness {
	AlwaysGrowsWhenTypingAtEdges = 0,
	NeverGrowsWhenTypingAtEdges = 1,
	GrowsOnlyWhenTypingBefore = 2,
	GrowsOnlyWhenTypingAfter = 3,
}

/** End-of-line character preference for language edits. */
export const enum EndOfLineSequence {
	LF = 0,
	CRLF = 1,
}

export const enum EndOfLinePreference {
	TextDefined = 0,
	LF = 1,
	CRLF = 2,
}

/** Text snapshot consumed sequentially by model clients. */
export interface ITextSnapshot {
	read(): string | null;
}

export function isITextSnapshot(value: unknown): value is ITextSnapshot {
	return !!value && typeof (value as ITextSnapshot).read === 'function';
}

/**
 * Editor-facing text model contract. The interface grows with supported editor
 * capabilities while preserving VS Code's ownership and method names.
 */
export interface ITextModel extends IDisposable {
	readonly uri: URI;
	readonly id: string;
	readonly isForSimpleWidget: boolean;
	readonly onWillDispose: Event<void>;
	readonly onDidChangeLanguage: Event<IModelLanguageChangedEvent>;
	mightContainRTL(): boolean;
	mightContainUnusualLineTerminators(): boolean;
	removeUnusualLineTerminators(): void;
	mightContainNonBasicASCII(): boolean;
	getLanguageId(): string;
	setLanguage(languageId: string | ILanguageSelection, source?: string): void;
	getVersionId(): number;
	setValue(newValue: string | ITextSnapshot): void;
	getValue(eol?: EndOfLinePreference, preserveBOM?: boolean): string;
	createSnapshot(preserveBOM?: boolean): ITextSnapshot;
	getValueLength(eol?: EndOfLinePreference, preserveBOM?: boolean): number;
	getValueInRange(range: IRange, eol?: EndOfLinePreference): string;
	getValueLengthInRange(range: IRange, eol?: EndOfLinePreference): number;
	getCharacterCountInRange(range: IRange, eol?: EndOfLinePreference): number;
	getLineCount(): number;
	getLineContent(lineNumber: number): string;
	getLineLength(lineNumber: number): number;
	getLinesContent(): string[];
	getEOL(): string;
	getEndOfLineSequence(): EndOfLineSequence;
	getLineMinColumn(lineNumber: number): number;
	getLineMaxColumn(lineNumber: number): number;
	getLineFirstNonWhitespaceColumn(lineNumber: number): number;
	getLineLastNonWhitespaceColumn(lineNumber: number): number;
	getFullModelRange(): Range;
	modifyPosition(position: IPosition, offset: number): Position;
	getOffsetAt(position: IPosition): number;
	getPositionAt(offset: number): Position;
	validatePosition(position: IPosition): Position;
	validateRange(range: IRange): Range;
	isValidRange(range: IRange): boolean;
	getLanguageIdAtPosition(lineNumber: number, column: number): string;
	canUndo(): boolean;
	canRedo(): boolean;
	normalizePosition(position: Position, affinity: PositionAffinity): Position;
	getLineIndentColumn(lineNumber: number): number;
}
import type { Event } from '../../base/common/event.js';
import type { IDisposable } from '../../base/common/lifecycle.js';
import type { URI } from '../../base/common/uri.js';
import type { IPosition, Position } from './core/position.js';
import type { IRange, Range } from './core/range.js';
import type { ILanguageSelection } from './languages/language.js';
import type { IModelLanguageChangedEvent } from './textModelEvents.js';
