import { Emitter, type Event } from "../../../base/common/event.js";
import { DisposableOwner, DisposableSlot, type IDisposable } from "../../../base/common/lifecycle.js";
import { canCoalesceHistoryEdits, canReplaceHistoryEdits, coalesceHistoryUndoEdits, normalizeInverseEdits, replaceHistoryUndoEdits, type OffsetTextEdit } from "./historyCoalescing.js";
import { PieceTreeTextBuffer } from "./pieceTreeTextBuffer/pieceTreeTextBuffer.js";
import { normalizeTextLineEndings, TextEditHistoryGroup, TextEditHistoryMergeMode, TextModelChangeReason, TextPosition, TextRange, TextLength, type ISingleEditOperation, type TextEdit, type TextModelChange, type TextModelContentChange, type TextSnapshot } from "../core/text.js";
import { TextModelHistory, type TextModelHistoryEntry } from "./editStack.js";
import { TrackedRangeCollection, type TrackedRange, type TrackedRangeStickiness } from "./trackedRange.js";
import { classifyTextModelSize, type TextModelLargeFilePolicy } from "./textModelLargeFile.js";

interface OffsetEdit extends OffsetTextEdit {}

interface PreparedEdit extends OffsetEdit {
	readonly range: TextRange;
	readonly replacedText: string;
}

interface CommitContext {
	readonly reason: TextModelChangeReason;
	readonly transactionId?: number;
}

export interface TextModelHistoryLimit {
	readonly transactions?: number;
	readonly textUnits?: number;
}

/** Schedules cancellable TextModel maintenance outside the edit transaction. */
export type TextModelMaintenanceScheduler = (callback: () => void) => IDisposable;

/** Controls how one TextModel runs optional storage maintenance. */
export interface TextModelMaintenanceOptions {
	readonly schedule: TextModelMaintenanceScheduler;
}

export interface TextModelOptions {
	readonly historyLimit?: TextModelHistoryLimit;
	/** Product-owned scheduling for non-semantic piece-tree maintenance. */
	readonly maintenance?: TextModelMaintenanceOptions;
}

export interface TextEditOptions {
	readonly historyGroup?: TextEditHistoryGroup;
	readonly historyMergeMode?: TextEditHistoryMergeMode;
}

const DEFAULT_HISTORY_TRANSACTIONS = 1_000;
const DEFAULT_HISTORY_TEXT_UNITS = 16 * 1_024 * 1_024;

/**
 * Zeta's canonical mutable text document.
 *
 * The model owns normalized LF text, versioning, atomic non-overlapping edit
 * transactions, transaction-level undo/redo, and generic tracked ranges. It
 * deliberately has no DOM, URI, persistence, language, selection-instance, or
 * presentation dependency.
 */
export class TextModel extends DisposableOwner {
	private readonly changeEmitter = this.own(new Emitter<TextModelChange>());
	private readonly trackedRanges = this.own(new TrackedRangeCollection(
		offset => this.positionAt(offset),
	));
	private readonly history: TextModelHistory;
	private readonly maintenance: TextModelMaintenanceOptions | undefined;
	private readonly pendingMaintenance = this.own(new DisposableSlot<IDisposable>());
	private buffer: PieceTreeTextBuffer;
	readonly largeFile: TextModelLargeFilePolicy;
	private nextTransactionId = 1;
	private _version = 1;
	private disposed = false;

	readonly onDidChange: Event<TextModelChange> = this.changeEmitter.event;

	constructor(initialText = "", options: TextModelOptions = {}) {
		super();
		const historyTransactionLimit = readHistoryLimit(
			options.historyLimit?.transactions,
			DEFAULT_HISTORY_TRANSACTIONS,
			"historyLimit.transactions",
		);
		const historyTextUnitLimit = readHistoryLimit(
			options.historyLimit?.textUnits,
			DEFAULT_HISTORY_TEXT_UNITS,
			"historyLimit.textUnits",
		);
		this.maintenance = readMaintenanceOptions(options.maintenance);
		this.history = new TextModelHistory(
			historyTransactionLimit,
			historyTextUnitLimit,
		);
		this.buffer = new PieceTreeTextBuffer(normalizeTextLineEndings(initialText));
		this.largeFile = classifyTextModelSize(this.buffer.length, this.buffer.lineCount);
		this.defer(() => {
			this.disposed = true;
			this.history.dispose();
			this.buffer = new PieceTreeTextBuffer("");
		});
	}

	get version(): number {
		this.ensureAlive();
		return this._version;
	}

	get lineCount(): number {
		this.ensureAlive();
		return this.buffer.lineCount;
	}

	get length(): number {
		this.ensureAlive();
		return this.buffer.length;
	}

	/** The document length in the same line/column algebra used by core edits. */
	get textLength(): TextLength {
		this.ensureAlive();
		return new TextLength(this.buffer.lineCount - 1, this.buffer.getLineLength(this.buffer.lineCount - 1));
	}

	get canUndo(): boolean {
		this.ensureAlive();
		return this.history.canUndo;
	}

	get canRedo(): boolean {
		this.ensureAlive();
		return this.history.canRedo;
	}

	getText(): string {
		this.ensureAlive();
		return this.buffer.getText();
	}

	createSnapshot(): TextSnapshot {
		this.ensureAlive();
		const version = this._version;
		const snapshot = this.buffer.createSnapshot();
		return Object.freeze({
			version,
			length: snapshot.length,
			lineCount: snapshot.lineCount,
			getText: () => snapshot.getText(),
			getTextBetweenOffsets: (
				startOffset: number,
				endOffset: number,
			) => snapshot.getTextBetweenOffsets(startOffset, endOffset),
		});
	}

	getTextInRange(range: TextRange): string {
		this.ensureAlive();
		return this.buffer.getTextInRange(
			this.offsetAt(range.start),
			this.offsetAt(range.end),
		);
	}

	getLineContent(lineIndex: number): string {
		this.ensureAlive();
		return this.buffer.getLineContent(lineIndex);
	}

	getLineLength(lineIndex: number): number {
		this.ensureAlive();
		return this.buffer.getLineLength(lineIndex);
	}

	offsetAt(position: TextPosition): number {
		this.ensureAlive();
		return this.buffer.offsetAt(
			position.lineIndex,
			position.columnIndex,
		);
	}

	positionAt(offset: number): TextPosition {
		this.ensureAlive();
		const position = this.buffer.positionAt(offset);
		return TextPosition.at(
			position.lineIndex,
			position.columnIndex,
		);
	}

	trackRange(
		range: TextRange,
		stickiness: TrackedRangeStickiness,
	): TrackedRange {
		this.ensureAlive();
		return this.trackedRanges.add(
			this.offsetAt(range.start),
			this.offsetAt(range.end),
			stickiness,
		);
	}

	beginHistoryRevision(historyGroup: TextEditHistoryGroup): void {
		this.ensureAlive();
		this.history.beginRevision(historyGroup);
	}

	finishHistoryRevision(historyGroup: TextEditHistoryGroup): boolean {
		this.ensureAlive();
		const entry = this.history.getRevisionEntry(historyGroup);
		if (entry && this.offsetEditsAreNoOps(entry.edits)) {
			this.history.discardRevision(historyGroup);
			return false;
		} else {
			return this.history.finishRevision(historyGroup);
		}
	}

	cancelHistoryRevision(
		historyGroup: TextEditHistoryGroup,
	): TextModelChange | undefined {
		this.ensureAlive();
		const entry = this.history.cancelRevision(historyGroup);
		if (!entry || this.offsetEditsAreNoOps(entry.edits)) return undefined;
		const result = this.commitOffsetEdits(entry.edits, {
			reason: TextModelChangeReason.HistoryCancellation,
			transactionId: entry.transactionId,
		});
		if (!result) {
			throw new Error("History revision contained an empty transaction");
		}
		this.changeEmitter.fire(result.change);
		return result.change;
	}

	/**
	 * Atomically applies replacements expressed against the current document.
	 *
	 * Input order is irrelevant. Overlapping replacements, including two
	 * insertions at the same position, are rejected before any mutation.
	 */
	applyOperations(operations: readonly ISingleEditOperation[], options: TextEditOptions = {}): TextModelChange | undefined {
		if (!Array.isArray(operations)) throw new TypeError("Edit operations must be an array");
		return this.applyEdits(operations.map(operation => ({ range: operation.range, text: operation.text ?? "" })), options);
	}

	applyEdits(
		edits: readonly TextEdit[],
		options: TextEditOptions = {},
	): TextModelChange | undefined {
		this.ensureAlive();
		const historyMergeMode =
			options.historyMergeMode ??
			TextEditHistoryMergeMode.Sequential;
		if (
			historyMergeMode !== TextEditHistoryMergeMode.Sequential &&
			historyMergeMode !== TextEditHistoryMergeMode.ReplacePrevious
		) {
			throw new TypeError("Unknown text edit history merge mode");
		}
		if (
			historyMergeMode === TextEditHistoryMergeMode.ReplacePrevious &&
			(!options.historyGroup ||
				!this.history.isRevisionActive(options.historyGroup))
		) {
			throw new Error(
				"ReplacePrevious requires an active history revision",
			);
		}
		const offsetEdits = edits.map(edit => {
			if (typeof edit.text !== "string") {
				throw new TypeError("TextEdit.text must be a string");
			}
			return {
				startOffset: this.offsetAt(edit.range.start),
				endOffset: this.offsetAt(edit.range.end),
				text: normalizeTextLineEndings(edit.text),
			};
		});
		const sortedOffsetEdits = [...offsetEdits].sort(compareOffsetEdits);
		const coalescingEntry = this.findCoalescingEntry(
			sortedOffsetEdits,
			options.historyGroup,
			historyMergeMode,
		);
		const result = this.commitOffsetEdits(
			offsetEdits,
			{
				reason: TextModelChangeReason.Edit,
				transactionId: coalescingEntry?.transactionId,
			},
		);
		if (!result) return undefined;
		this.history.prepareForEdit(options.historyGroup);
		this.history.clearRedo();
		if (coalescingEntry) {
			const mergedEdits =
				historyMergeMode === TextEditHistoryMergeMode.ReplacePrevious
					? replaceHistoryUndoEdits(
						coalescingEntry.edits,
						result.inverseEdits,
					)
					: coalesceHistoryUndoEdits(
						coalescingEntry.edits,
						sortedOffsetEdits,
						result.inverseEdits,
					);
			this.history.replaceUndoEntry(coalescingEntry, mergedEdits);
		} else {
			this.history.pushUndo(
				result.inverseEdits,
				result.change.transactionId,
				options.historyGroup,
			);
		}
		this.changeEmitter.fire(result.change);
		return result.change;
	}

	/** Clears edit history and replaces changed content as a non-undoable document reset. */
	reset(text: string): TextModelChange | undefined {
		this.ensureAlive();
		if (typeof text !== "string") {
			throw new TypeError("TextModel reset text must be a string");
		}
		const result = this.commitOffsetEdits([{
			startOffset: 0,
			endOffset: this.buffer.length,
			text: normalizeTextLineEndings(text),
		}], { reason: TextModelChangeReason.Reset });
		this.history.reset();
		if (!result) return undefined;
		this.changeEmitter.fire(result.change);
		return result.change;
	}

	undo(): TextModelChange | undefined {
		this.ensureAlive();
		this.history.prepareForEdit(undefined);
		const entry = this.history.takeUndo();
		if (!entry) return undefined;
		const result = this.commitOffsetEdits(
			entry.edits,
			{
				reason: TextModelChangeReason.Undo,
				transactionId: entry.transactionId,
			},
		);
		if (!result) {
			throw new Error("Undo history contained an empty transaction");
		}
		this.history.pushRedo(
			result.inverseEdits,
			entry.transactionId,
			entry.historyGroup,
		);
		this.changeEmitter.fire(result.change);
		return result.change;
	}

	redo(): TextModelChange | undefined {
		this.ensureAlive();
		this.history.prepareForEdit(undefined);
		const entry = this.history.takeRedo();
		if (!entry) return undefined;
		const result = this.commitOffsetEdits(
			entry.edits,
			{
				reason: TextModelChangeReason.Redo,
				transactionId: entry.transactionId,
			},
		);
		if (!result) {
			throw new Error("Redo history contained an empty transaction");
		}
		this.history.pushUndo(
			result.inverseEdits,
			entry.transactionId,
			entry.historyGroup,
		);
		this.changeEmitter.fire(result.change);
		return result.change;
	}

	private findCoalescingEntry(
		edits: readonly OffsetEdit[],
		historyGroup: TextEditHistoryGroup | undefined,
		historyMergeMode: TextEditHistoryMergeMode,
	): TextModelHistoryEntry | undefined {
		return this.history.findUndoEntry(
			historyGroup,
			previous =>
				historyMergeMode === TextEditHistoryMergeMode.ReplacePrevious
					? canReplaceHistoryEdits(previous.edits, edits)
					: canCoalesceHistoryEdits(previous.edits, edits),
		);
	}

	private commitOffsetEdits(
		edits: readonly OffsetEdit[],
		context: CommitContext,
	): {
		readonly change: TextModelChange;
		readonly inverseEdits: OffsetEdit[];
	} | undefined {
		const prepared = this.prepareEdits(edits);
		if (prepared.length === 0) return undefined;
		const transactionId =
			context.transactionId ??
			this.nextTransactionId++;

		const inverseEdits: OffsetEdit[] = [];
		let cumulativeDelta = 0;
		for (const edit of prepared) {
			const newStartOffset = edit.startOffset + cumulativeDelta;
			inverseEdits.push({
				startOffset: newStartOffset,
				endOffset: newStartOffset + edit.text.length,
				text: edit.replacedText,
			});
			cumulativeDelta +=
				edit.text.length - (edit.endOffset - edit.startOffset);
		}

		const changes = Object.freeze(
			prepared.map<TextModelContentChange>(edit => Object.freeze({
				range: edit.range,
				rangeOffset: edit.startOffset,
				rangeLength: edit.endOffset - edit.startOffset,
				text: edit.text,
			})),
		);
		for (let index = prepared.length - 1; index >= 0; index -= 1) {
			const edit = prepared[index];
			this.buffer.replace(
				edit.startOffset,
				edit.endOffset,
				edit.text,
			);
		}
		this.trackedRanges.acceptChanges(changes);
		this.scheduleMaintenance();

		this._version += 1;
		const change = Object.freeze<TextModelChange>({
			version: this._version,
			transactionId,
			reason: context.reason,
			changes,
		});
		return {
			change,
			inverseEdits: normalizeInverseEdits(inverseEdits),
		};
	}

	private scheduleMaintenance(): void {
		if (!this.buffer.needsCompaction()) return;
		const maintenance = this.maintenance;
		if (!maintenance) {
			this.buffer.compact();
			return;
		}
		if (this.pendingMaintenance.value) return;
		let pending: IDisposable;
		let ranSynchronously = false;
		try {
			pending = maintenance.schedule(() => {
				ranSynchronously = true;
				this.pendingMaintenance.clear();
				if (!this.disposed) this.buffer.compactIfNeeded();
			});
			if (!pending || typeof pending.dispose !== "function") {
				throw new TypeError("TextModel maintenance scheduler must return a disposable");
			}
		} catch {
			this.buffer.compact();
			return;
		}
		if (ranSynchronously) {
			pending.dispose();
			return;
		}
		this.pendingMaintenance.replace(pending);
	}

	private prepareEdits(edits: readonly OffsetEdit[]): PreparedEdit[] {
		const sorted = edits.map(edit => {
			this.assertOffsetRange(edit);
			return {
				...edit,
				text: normalizeTextLineEndings(edit.text),
			};
		}).sort(compareOffsetEdits);

		for (let index = 1; index < sorted.length; index += 1) {
			const previous = sorted[index - 1];
			const current = sorted[index];
			const ambiguousSharedStart =
				current.startOffset === previous.startOffset &&
				(current.startOffset === current.endOffset ||
					previous.startOffset === previous.endOffset);
			if (
				current.startOffset < previous.endOffset ||
				ambiguousSharedStart
			) {
				throw new RangeError("Text edits must not overlap");
			}
		}

		return sorted.flatMap<PreparedEdit>(edit => {
			const replacedText = this.buffer.getTextInRange(
				edit.startOffset,
				edit.endOffset,
			);
			if (replacedText === edit.text) return [];
			return [{
				...edit,
				range: TextRange.from(
					this.positionAt(edit.startOffset),
					this.positionAt(edit.endOffset),
				),
				replacedText,
			}];
		});
	}

	private assertOffsetRange(edit: OffsetEdit): void {
		if (
			!Number.isSafeInteger(edit.startOffset) ||
			!Number.isSafeInteger(edit.endOffset) ||
			edit.startOffset < 0 ||
			edit.endOffset < edit.startOffset ||
			edit.endOffset > this.buffer.length
		) {
			throw new RangeError(
				`Text edit offsets must satisfy 0 <= start <= end <= ${this.buffer.length}`,
			);
		}
	}

	private offsetEditsAreNoOps(edits: readonly OffsetEdit[]): boolean {
		return edits.every(edit =>
			this.buffer.getTextInRange(
				edit.startOffset,
				edit.endOffset,
			) === edit.text,
		);
	}

	private ensureAlive(): void {
		if (this.disposed) {
			throw new ReferenceError("TextModel is already disposed");
		}
	}
}

function compareOffsetEdits(left: OffsetEdit, right: OffsetEdit): number {
	return left.startOffset - right.startOffset ||
		left.endOffset - right.endOffset;
}

function readHistoryLimit(
	value: number | undefined,
	defaultValue: number,
	name: string,
): number {
	const resolved = value ?? defaultValue;
	if (!Number.isSafeInteger(resolved) || resolved < 0) {
		throw new RangeError(`${name} must be a non-negative safe integer`);
	}
	return resolved;
}

function readMaintenanceOptions(value: TextModelMaintenanceOptions | undefined): TextModelMaintenanceOptions | undefined {
	if (value === undefined) return undefined;
	if (!value || typeof value.schedule !== "function") {
		throw new TypeError("TextModel maintenance requires a scheduler");
	}
	return Object.freeze({ schedule: value.schedule });
}
