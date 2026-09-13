import { type Event } from '../../../base/common/event.js';
import { type TextModelChange } from '../core/textChange.js';

/** A half-open range of visual-line indexes shared by layout and view-model code. */
export interface EditorLineRange {
	readonly startLineIndex: number;
	readonly endLineIndexExclusive: number;
}

/** Supplies the current visual-line collection to the common layout. */
export interface EditorViewportLineSource {
	readonly lineCount: number;
	readonly onDidChange: Event<void>;
}

/** Minimal model contract consumed by the DOM-free viewport layout. */
export interface EditorViewportModelSource {
	readonly lineCount: number;
	readonly version: number;
	readonly onDidChangeContent: Event<TextModelChange>;
}

/** Batch mutation boundary used by view-layout custom line-height owners. */
export interface EditorLineHeightChangeAccessor {
	insertOrChangeCustomLineHeight(decorationId: string, startLineNumber: number, endLineNumber: number, lineHeight: number): void;
	removeCustomLineHeight(decorationId: string): void;
}

/** Immutable geometry for one block of vertical space between visual lines. */
export interface EditorViewZoneLayout {
	readonly id: string;
	readonly afterLineIndex: number;
	readonly top: number;
	readonly heightInPixels: number;
}

/** The scroll coordinates exchanged between the view-model and the browser view. */
export interface EditorScrollPosition {
	readonly left: number;
	readonly top: number;
}
