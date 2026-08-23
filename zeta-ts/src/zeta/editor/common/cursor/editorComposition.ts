import { type EditorEditCommand, type TextSelectionOffsets } from "../commands/editorEditCommand.js";
import { normalizeTextLineEndings, TextRange, type TextModelChange } from "../core/text.js";
import { TextModel } from "../model/textModel.js";

export interface EditorCompositionUpdate {
	readonly text: string;
	readonly selection: TextSelectionOffsets;
}

interface EditorCompositionHost {
	isActive(): boolean;
	assertActive(): void;
	apply(command: EditorEditCommand): TextModelChange | undefined;
	commit(): void;
	cancel(): TextModelChange | undefined;
}

/**
 * One single-selection IME composition.
 *
 * Updates revise the same protected undo step. Commit keeps that step, while
 * cancel restores the pre-composition text and removes the step.
 */
export class EditorCompositionSession {
	private currentEndOffset: number;
	private closed = false;

	constructor(
		private readonly model: TextModel,
		private readonly startOffset: number,
		endOffset: number,
		private readonly host: EditorCompositionHost,
	) {
		this.currentEndOffset = endOffset;
	}

	get active(): boolean {
		return !this.closed && this.host.isActive();
	}

	/** The model range currently occupied by provisional composition text. */
	get currentRange(): TextRange {
		this.ensureActive();
		this.host.assertActive();
		return TextRange.from(
			this.model.positionAt(this.startOffset),
			this.model.positionAt(this.currentEndOffset),
		);
	}

	update(update: EditorCompositionUpdate): TextModelChange | undefined {
		this.ensureActive();
		this.host.assertActive();
		if (typeof update.text !== "string") {
			throw new TypeError("EditorCompositionUpdate.text must be a string");
		}
		const text = normalizeTextLineEndings(update.text);
		validateRelativeSelection(update.selection, text.length);
		const change = this.host.apply({
			edits: [{
				range: TextRange.from(
					this.model.positionAt(this.startOffset),
					this.model.positionAt(this.currentEndOffset),
				),
				text,
			}],
			selectionsAfter: [{
				anchorOffset:
					this.startOffset + update.selection.anchorOffset,
				activeOffset:
					this.startOffset + update.selection.activeOffset,
			}],
			primarySelectionIndex: 0,
		});
		this.host.assertActive();
		this.currentEndOffset = this.startOffset + text.length;
		return change;
	}

	commit(): void {
		this.ensureActive();
		this.host.assertActive();
		this.closed = true;
		this.host.commit();
	}

	cancel(): TextModelChange | undefined {
		this.ensureActive();
		this.host.assertActive();
		this.closed = true;
		return this.host.cancel();
	}

	private ensureActive(): void {
		if (this.closed) {
			throw new ReferenceError("Editor composition is already closed");
		}
	}
}

function validateRelativeSelection(
	selection: TextSelectionOffsets,
	textLength: number,
): void {
	assertRelativeOffset(
		selection.anchorOffset,
		textLength,
		"selection.anchorOffset",
	);
	assertRelativeOffset(
		selection.activeOffset,
		textLength,
		"selection.activeOffset",
	);
}

function assertRelativeOffset(
	offset: number,
	textLength: number,
	name: string,
): void {
	if (
		!Number.isSafeInteger(offset) ||
		offset < 0 ||
		offset > textLength
	) {
		throw new RangeError(`${name} must be between 0 and ${textLength}`);
	}
}
