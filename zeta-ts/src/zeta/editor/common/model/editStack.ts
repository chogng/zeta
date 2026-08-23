import { type OffsetTextEdit } from "./historyCoalescing.js";
import { TextEditHistoryGroup } from "../core/text.js";

export interface TextModelHistoryEntry {
	readonly edits: readonly OffsetTextEdit[];
	readonly textUnits: number;
	readonly transactionId: number;
	readonly historyGroup: TextEditHistoryGroup | undefined;
}

export class TextModelHistory {
	private readonly undoStack: TextModelHistoryEntry[] = [];
	private readonly redoStack: TextModelHistoryEntry[] = [];
	private historyTextUnits = 0;
	private protectedGroup: TextEditHistoryGroup | undefined;

	constructor(
		private readonly transactionLimit: number,
		private readonly textUnitLimit: number,
	) {}

	get canUndo(): boolean {
		return this.undoStack.length > 0;
	}

	get canRedo(): boolean {
		return this.redoStack.length > 0;
	}

	isRevisionActive(historyGroup: TextEditHistoryGroup): boolean {
		return this.protectedGroup === historyGroup;
	}

	prepareForEdit(historyGroup: TextEditHistoryGroup | undefined): void {
		if (this.protectedGroup && this.protectedGroup !== historyGroup) {
			this.protectedGroup = undefined;
			this.trim();
		}
	}

	beginRevision(historyGroup: TextEditHistoryGroup): void {
		if (this.protectedGroup) {
			throw new Error("A history revision is already active");
		}
		this.protectedGroup = historyGroup;
	}

	finishRevision(historyGroup: TextEditHistoryGroup): boolean {
		this.assertProtectedGroup(historyGroup);
		this.protectedGroup = undefined;
		this.trim();
		return this.undoStack.some(
			entry => entry.historyGroup === historyGroup,
		);
	}

	getRevisionEntry(
		historyGroup: TextEditHistoryGroup,
	): TextModelHistoryEntry | undefined {
		this.assertProtectedGroup(historyGroup);
		const entry = this.undoStack[this.undoStack.length - 1];
		return entry?.historyGroup === historyGroup ? entry : undefined;
	}

	discardRevision(historyGroup: TextEditHistoryGroup): void {
		this.cancelRevision(historyGroup);
	}

	cancelRevision(
		historyGroup: TextEditHistoryGroup,
	): TextModelHistoryEntry | undefined {
		this.assertProtectedGroup(historyGroup);
		this.protectedGroup = undefined;
		const entry = this.undoStack[this.undoStack.length - 1];
		if (!entry) return undefined;
		if (entry.historyGroup !== historyGroup) {
			throw new Error("The active history revision is not the latest undo step");
		}
		return this.takeUndo();
	}

	findUndoEntry(
		historyGroup: TextEditHistoryGroup | undefined,
		accepts: (entry: TextModelHistoryEntry) => boolean,
	): TextModelHistoryEntry | undefined {
		if (!historyGroup || this.redoStack.length > 0) return undefined;
		const previous = this.undoStack[this.undoStack.length - 1];
		return previous?.historyGroup === historyGroup && accepts(previous)
			? previous
			: undefined;
	}

	replaceUndoEntry(
		previous: TextModelHistoryEntry,
		edits: readonly OffsetTextEdit[],
	): void {
		if (this.undoStack[this.undoStack.length - 1] !== previous) {
			throw new Error("Only the latest undo entry can be replaced");
		}
		const replacement = createEntry(
			edits,
			previous.transactionId,
			previous.historyGroup,
		);
		this.historyTextUnits += replacement.textUnits - previous.textUnits;
		this.undoStack[this.undoStack.length - 1] = replacement;
		this.trim();
	}

	pushUndo(
		edits: readonly OffsetTextEdit[],
		transactionId: number,
		historyGroup: TextEditHistoryGroup | undefined,
	): void {
		this.push(this.undoStack, edits, transactionId, historyGroup);
		this.trim();
	}

	pushRedo(
		edits: readonly OffsetTextEdit[],
		transactionId: number,
		historyGroup: TextEditHistoryGroup | undefined,
	): void {
		this.push(this.redoStack, edits, transactionId, historyGroup);
		this.trim();
	}

	takeUndo(): TextModelHistoryEntry | undefined {
		return this.take(this.undoStack);
	}

	takeRedo(): TextModelHistoryEntry | undefined {
		return this.take(this.redoStack);
	}

	clearRedo(): void {
		this.clear(this.redoStack);
	}

	reset(): void {
		this.undoStack.length = 0;
		this.redoStack.length = 0;
		this.historyTextUnits = 0;
		this.protectedGroup = undefined;
	}

	dispose(): void {
		this.reset();
	}

	private push(
		stack: TextModelHistoryEntry[],
		edits: readonly OffsetTextEdit[],
		transactionId: number,
		historyGroup: TextEditHistoryGroup | undefined,
	): void {
		const entry = createEntry(edits, transactionId, historyGroup);
		stack.push(entry);
		this.historyTextUnits += entry.textUnits;
	}

	private take(
		stack: TextModelHistoryEntry[],
	): TextModelHistoryEntry | undefined {
		const entry = stack.pop();
		if (entry) this.historyTextUnits -= entry.textUnits;
		return entry;
	}

	private clear(stack: TextModelHistoryEntry[]): void {
		for (const entry of stack) {
			this.historyTextUnits -= entry.textUnits;
		}
		stack.length = 0;
	}

	private trim(): void {
		while (
			this.undoStack.length + this.redoStack.length >
				this.transactionLimit ||
			this.historyTextUnits > this.textUnitLimit
		) {
			if (
				this.protectedGroup &&
				this.undoStack.length === 1 &&
				this.undoStack[0].historyGroup === this.protectedGroup
			) {
				break;
			}
			const stack = this.undoStack.length > 0
				? this.undoStack
				: this.redoStack;
			const [removed] = stack.splice(0, 1);
			this.historyTextUnits -= removed.textUnits;
		}
	}

	private assertProtectedGroup(historyGroup: TextEditHistoryGroup): void {
		if (this.protectedGroup !== historyGroup) {
			throw new Error("The history revision is no longer active");
		}
	}
}

function createEntry(
	edits: readonly OffsetTextEdit[],
	transactionId: number,
	historyGroup: TextEditHistoryGroup | undefined,
): TextModelHistoryEntry {
	return {
		edits,
		textUnits: edits.reduce(
			(total, edit) => total + edit.text.length,
			0,
		),
		transactionId,
		historyGroup,
	};
}
