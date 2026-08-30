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
	getLanguageId(): string;
	setLanguage(languageId: string | ILanguageSelection, source?: string): void;
	getVersionId(): number;
	getValue(): string;
	getValueInRange(range: IRange): string;
	getLineCount(): number;
	getLineContent(lineNumber: number): string;
	getLineLength(lineNumber: number): number;
	getLineMaxColumn(lineNumber: number): number;
	getFullModelRange(): Range;
	getOffsetAt(position: IPosition): number;
	getPositionAt(offset: number): Position;
	validatePosition(position: IPosition): Position;
	validateRange(range: IRange): Range;
}
import type { Event } from '../../base/common/event.js';
import type { IDisposable } from '../../base/common/lifecycle.js';
import type { URI } from '../../base/common/uri.js';
import type { IPosition, Position } from './core/position.js';
import type { IRange, Range } from './core/range.js';
import type { ILanguageSelection } from './languages/language.js';
import type { IModelLanguageChangedEvent } from './textModelEvents.js';
