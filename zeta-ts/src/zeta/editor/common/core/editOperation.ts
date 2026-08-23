import { TextPosition } from "./position.js";
import { TextRange } from "./range.js";

/** A single pre-transaction replacement consumed by the text model. */
export interface ISingleEditOperation {
	readonly range: TextRange;
	readonly text: string | null;
	readonly forceMoveMarkers?: boolean;
}

/** One replacement in the pre-transaction document. */
export interface TextEdit {
	readonly range: TextRange;
	readonly text: string;
}

/** Factory helpers for the editor's canonical single-edit operation shape. */
export class EditOperation {
	static insert(position: TextPosition, text: string): ISingleEditOperation { return { range: TextRange.emptyAt(position), text, forceMoveMarkers: true }; }
	static delete(range: TextRange): ISingleEditOperation { return { range, text: null }; }
	static replace(range: TextRange, text: string | null): ISingleEditOperation { return { range, text }; }
	static replaceMove(range: TextRange, text: string | null): ISingleEditOperation { return { range, text, forceMoveMarkers: true }; }
}

/**
 * Nominal identity used only while consecutive compatible edits may share one
 * undo step.
 */
export class TextEditHistoryGroup {
	private readonly identity = undefined;

	private constructor() {
		Object.freeze(this);
	}

	static create(): TextEditHistoryGroup {
		return new TextEditHistoryGroup();
	}
}

/** Selects how a compatible edit updates the latest grouped undo step. */
export enum TextEditHistoryMergeMode {
	Sequential = "sequential",
	ReplacePrevious = "replacePrevious",
}
