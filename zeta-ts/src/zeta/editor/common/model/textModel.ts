import { Emitter, type Event } from "../../../base/common/event.js";
import { Disposable, MutableDisposable, type IDisposable, toDisposable } from "../../../base/common/lifecycle.js";
import { canCoalesceHistoryEdits, canReplaceHistoryEdits, coalesceHistoryUndoEdits, normalizeInverseEdits, replaceHistoryUndoEdits, type OffsetTextEdit } from "./historyCoalescing.js";
import type { TextBuffer } from "./textBuffer.js";
import { createTextBuffer } from "./textBufferFactory.js";
import { normalizeTextLineEndings, TextEditHistoryGroup, TextEditHistoryMergeMode, TextModelChangeReason, TextPosition, TextRange, TextLength, type ISingleEditOperation, type TextEdit, type TextModelChange, type TextModelContentChange, type TextSnapshot } from "../core/text.js";
import { TextModelHistory, type TextModelHistoryEntry } from "./editStack.js";
import { TrackedRangeCollection, type TrackedRange, type TrackedRangeStickiness } from "./trackedRange.js";
import { classifyTextModelSize, type TextModelLargeFilePolicy } from "./textModelLargeFile.js";
import type { DocumentSelection } from "../core/documentSelection.js";
import type { DocumentMark, DocumentNode } from "./document.js";
import type { DocumentHistoryEntries } from "./documentHistory.js";
import type { DocumentPluginKey } from "./documentPlugin.js";
import type { DocumentSchema } from "./documentSchema.js";
import type { DocumentTransaction } from "./documentTransaction.js";
import { TextModelBlockState, TextModelRemoteHistoryPolicy, type TextModelBlockChange, type TextModelBlockOptions, type TextModelPluginDecorationSource } from "./textModelBlockState.js";
import { projectDocumentToLines } from "./lineDocumentProjection.js";
import { createLineDocumentSnapshot, linePoint, type LineDocumentSnapshot, type LineId, type LinePoint, type LineSemanticAttributes } from "./lineDocument.js";

interface OffsetEdit extends OffsetTextEdit {}

interface PreparedEdit extends OffsetEdit {
	readonly range: TextRange;
	readonly replacedText: string;
}

interface CommitContext {
	readonly reason: TextModelChangeReason;
	readonly transactionId?: number;
	readonly lineIds?: readonly LineId[];
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
	/** Schema-backed blocks used by document profiles. */
	readonly blocks?: TextModelBlockInitialization;
	/** Stable identities restored by a rich-document codec. Plain text codecs omit this. */
	readonly lineIds?: readonly LineId[];
	/** Document-level metadata such as a code file's languageId. */
	readonly metadata?: LineSemanticAttributes;
	/** Identity source for logical lines created by text edits. */
	readonly lineIdGenerator?: () => LineId;
}

export interface TextModelBlockInitialization extends TextModelBlockOptions {
	readonly schema: DocumentSchema;
	readonly document?: DocumentNode;
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
 * transactions, transaction-level undo/redo, and generic tracked ranges.
 * Logical line identity and rich semantic stores remain part of this same model and version.
 * The model has no DOM, URI, persistence, language, or presentation dependency.
 */
export class TextModel extends Disposable {
	private readonly willDisposeEmitter = this._register(new Emitter<void>());
	private readonly changeEmitter = this._register(new Emitter<TextModelChange>());
	private readonly trackedRanges = this._register(new TrackedRangeCollection(
		offset => this.positionAt(offset),
	));
	private readonly history: TextModelHistory;
	private readonly maintenance: TextModelMaintenanceOptions | undefined;
	private readonly pendingMaintenance = this._register(new MutableDisposable<IDisposable>());
	private buffer: TextBuffer;
	private readonly blockState: TextModelBlockState | undefined;
	private readonly lineIdGenerator: () => LineId;
	private readonly issuedLineIds = new Set<LineId>();
	private nextGeneratedLineIdentity = 1;
	private readonly lineMetadata: LineSemanticAttributes;
	private plainLineIds: readonly LineId[] | undefined;
	private plainLineSnapshot: LineDocumentSnapshot | undefined;
	readonly largeFile: TextModelLargeFilePolicy;
	private nextTransactionId = 1;
	private _version = 1;

	readonly onDidChange: Event<TextModelChange> = this.changeEmitter.event;
	/** Fires once so registries can release model identity before teardown completes. */
	readonly onWillDispose: Event<void> = this.willDisposeEmitter.event;

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
		if (options.blocks && options.lineIds) throw new TypeError("Schema-backed TextModel line identities come from the document codec");
		if (options.lineIdGenerator !== undefined && typeof options.lineIdGenerator !== "function") {
			throw new TypeError("TextModel lineIdGenerator must be a function");
		}
		this.lineIdGenerator = options.lineIdGenerator ?? (() => `line:${this.nextGeneratedLineIdentity++}`);
		const blockDocument = options.blocks?.document ?? options.blocks?.schema.createDocument();
		const blockSnapshot = blockDocument && options.blocks ? projectDocumentToLines(options.blocks.schema, blockDocument) : undefined;
		const normalizedInitialText = blockSnapshot?.getText() ?? normalizeTextLineEndings(initialText);
		this.buffer = createTextBuffer(normalizedInitialText);
		if (!options.blocks) {
			this.plainLineIds = this.initializeLineIds(options.lineIds);
			this.plainLineSnapshot = this.createPlainLineSnapshot(options.metadata);
			this.lineMetadata = this.plainLineSnapshot.metadata;
		} else {
			this.lineMetadata = Object.freeze({});
		}
		this.largeFile = classifyTextModelSize(this.buffer.length, this.buffer.lineCount);
		this.blockState = options.blocks && blockDocument ? this._register(new TextModelBlockState(
			options.blocks.schema,
			blockDocument,
			options.blocks,
			{
				getVersion: () => this._version,
				commitText: text => this.commitBlockText(text),
				publishTextChange: change => this.changeEmitter.fire(change),
			},
		)) : undefined;
		this._register(toDisposable(() => {
			this.history.dispose();
			this.buffer = createTextBuffer("");
		}));
	}

	/** Creates one TextModel from schema-backed document content. */
	static create(schema: DocumentSchema, document = schema.createDocument(), options: TextModelBlockOptions = {}): TextModel {
		return new TextModel("", { blocks: { ...options, schema, document } });
	}

	/** Immutable line-first content snapshot at the current model version. */
	get lineDocument(): LineDocumentSnapshot {
		this.assertNotDisposed();
		return this.blockState?.snapshot ?? this.requirePlainLineSnapshot();
	}

	getLineId(lineIndex: number): LineId {
		this.assertNotDisposed();
		const line = this.lineDocument.lines.at(lineIndex);
		if (!line) throw new RangeError("Line index is outside the TextModel");
		return line.id;
	}

	getLineIndex(lineId: LineId): number {
		this.assertNotDisposed();
		const lineIndex = this.lineDocument.lines.indexOf(lineId);
		if (lineIndex < 0) throw new RangeError(`Line '${lineId}' does not exist in the TextModel`);
		return lineIndex;
	}

	linePointAt(position: TextPosition): LinePoint {
		this.assertNotDisposed();
		this.offsetAt(position);
		return linePoint(this.getLineId(position.lineIndex), position.columnIndex);
	}

	textPositionAt(point: LinePoint): TextPosition {
		this.assertNotDisposed();
		const lineIndex = this.getLineIndex(point.lineId);
		if (!Number.isSafeInteger(point.offset) || point.offset < 0 || point.offset > this.buffer.getLineLength(lineIndex)) {
			throw new RangeError("Line point offset is outside the TextModel");
		}
		return TextPosition.at(lineIndex, point.offset);
	}

	get schema(): DocumentSchema {
		return this.requireBlockState().schema;
	}

	get document(): DocumentNode {
		return this.requireBlockState().document;
	}

	get selection(): DocumentSelection | undefined {
		return this.requireBlockState().selection;
	}

	get storedMarks(): readonly DocumentMark[] | undefined {
		return this.requireBlockState().storedMarks;
	}

	get onDidChangeBlocks(): Event<TextModelBlockChange> {
		return this.requireBlockState().onDidChange;
	}

	get onDidChangeSelection(): Event<DocumentSelection | undefined> {
		return this.requireBlockState().onDidChangeSelection;
	}

	get onDidChangeStoredMarks(): Event<readonly DocumentMark[] | undefined> {
		return this.requireBlockState().onDidChangeStoredMarks;
	}

	get canUndoBlocks(): boolean {
		return this.requireBlockState().canUndo;
	}

	get canRedoBlocks(): boolean {
		return this.requireBlockState().canRedo;
	}

	dispatch(transaction: DocumentTransaction): TextModelBlockChange | undefined {
		return this.requireBlockState().dispatch(transaction);
	}

	dispatchRemote(transaction: DocumentTransaction, historyPolicy: TextModelRemoteHistoryPolicy = TextModelRemoteHistoryPolicy.Clear): TextModelBlockChange | undefined {
		return this.requireBlockState().dispatchRemote(transaction, historyPolicy);
	}

	rebaseHistory(mapper: (entries: DocumentHistoryEntries) => DocumentHistoryEntries): void {
		this.requireBlockState().rebaseHistory(mapper);
	}

	undoBlocks(): TextModelBlockChange | undefined {
		return this.requireBlockState().undo();
	}

	redoBlocks(): TextModelBlockChange | undefined {
		return this.requireBlockState().redo();
	}

	setSelection(selection: DocumentSelection | undefined): void {
		this.requireBlockState().setSelection(selection);
	}

	setStoredMarks(marks: readonly DocumentMark[] | undefined): void {
		this.requireBlockState().setStoredMarks(marks);
	}

	resetBlocks(document: DocumentNode): TextModelBlockChange | undefined {
		return this.requireBlockState().reset(document);
	}

	getPluginState<T>(key: DocumentPluginKey<T>): T | undefined {
		return this.requireBlockState().getPluginState(key);
	}

	getPluginDecorations(): readonly TextModelPluginDecorationSource[] {
		return this.requireBlockState().getPluginDecorations();
	}

	get version(): number {
		this.assertNotDisposed();
		return this._version;
	}

	get lineCount(): number {
		this.assertNotDisposed();
		return this.buffer.lineCount;
	}

	get length(): number {
		this.assertNotDisposed();
		return this.buffer.length;
	}

	/** The document length in the same line/column algebra used by core edits. */
	get textLength(): TextLength {
		this.assertNotDisposed();
		return new TextLength(this.buffer.lineCount - 1, this.buffer.getLineLength(this.buffer.lineCount - 1));
	}

	get canUndo(): boolean {
		this.assertNotDisposed();
		return this.history.canUndo;
	}

	get canRedo(): boolean {
		this.assertNotDisposed();
		return this.history.canRedo;
	}

	getText(): string {
		this.assertNotDisposed();
		return this.buffer.getText();
	}

	createSnapshot(): TextSnapshot {
		this.assertNotDisposed();
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
		this.assertNotDisposed();
		return this.buffer.getTextInRange(
			this.offsetAt(range.start),
			this.offsetAt(range.end),
		);
	}

	getLineContent(lineIndex: number): string {
		this.assertNotDisposed();
		return this.buffer.getLineContent(lineIndex);
	}

	getLineLength(lineIndex: number): number {
		this.assertNotDisposed();
		return this.buffer.getLineLength(lineIndex);
	}

	offsetAt(position: TextPosition): number {
		this.assertNotDisposed();
		return this.buffer.offsetAt(
			position.lineIndex,
			position.columnIndex,
		);
	}

	positionAt(offset: number): TextPosition {
		this.assertNotDisposed();
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
		this.assertNotDisposed();
		return this.trackedRanges.add(
			this.offsetAt(range.start),
			this.offsetAt(range.end),
			stickiness,
		);
	}

	beginHistoryRevision(historyGroup: TextEditHistoryGroup): void {
		this.assertNotDisposed();
		this.ensureDirectTextMutationAllowed();
		this.history.beginRevision(historyGroup);
	}

	finishHistoryRevision(historyGroup: TextEditHistoryGroup): boolean {
		this.assertNotDisposed();
		this.ensureDirectTextMutationAllowed();
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
		this.assertNotDisposed();
		this.ensureDirectTextMutationAllowed();
		const entry = this.history.cancelRevision(historyGroup);
		if (!entry || this.offsetEditsAreNoOps(entry.edits)) return undefined;
		const result = this.commitOffsetEdits(entry.edits, {
			reason: TextModelChangeReason.HistoryCancellation,
			transactionId: entry.transactionId,
			lineIds: entry.lineIds,
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
		this.assertNotDisposed();
		this.ensureDirectTextMutationAllowed();
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
				result.previousLineIds,
			);
		}
		this.changeEmitter.fire(result.change);
		return result.change;
	}

	/** Clears edit history and replaces changed content as a non-undoable document reset. */
	reset(text: string): TextModelChange | undefined {
		this.assertNotDisposed();
		this.ensureDirectTextMutationAllowed();
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
		this.assertNotDisposed();
		this.ensureDirectTextMutationAllowed();
		this.history.prepareForEdit(undefined);
		const entry = this.history.takeUndo();
		if (!entry) return undefined;
		const result = this.commitOffsetEdits(
			entry.edits,
			{
				reason: TextModelChangeReason.Undo,
				transactionId: entry.transactionId,
				lineIds: entry.lineIds,
			},
		);
		if (!result) {
			throw new Error("Undo history contained an empty transaction");
		}
		this.history.pushRedo(
			result.inverseEdits,
			entry.transactionId,
			entry.historyGroup,
			result.previousLineIds,
		);
		this.changeEmitter.fire(result.change);
		return result.change;
	}

	redo(): TextModelChange | undefined {
		this.assertNotDisposed();
		this.ensureDirectTextMutationAllowed();
		this.history.prepareForEdit(undefined);
		const entry = this.history.takeRedo();
		if (!entry) return undefined;
		const result = this.commitOffsetEdits(
			entry.edits,
			{
				reason: TextModelChangeReason.Redo,
				transactionId: entry.transactionId,
				lineIds: entry.lineIds,
			},
		);
		if (!result) {
			throw new Error("Redo history contained an empty transaction");
		}
		this.history.pushUndo(
			result.inverseEdits,
			entry.transactionId,
			entry.historyGroup,
			result.previousLineIds,
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
		readonly previousLineIds: readonly LineId[] | undefined;
	} | undefined {
		const prepared = this.prepareEdits(edits);
		if (prepared.length === 0) return undefined;
		const previousLineIds = this.plainLineIds;
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
		if (this.plainLineIds) {
			this.plainLineIds = context.lineIds === undefined ? this.mapLineIds(prepared) : this.validateCommittedLineIds(context.lineIds);
			this.plainLineSnapshot = undefined;
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
			previousLineIds,
		};
	}

	private scheduleMaintenance(): void {
		if (!this.buffer.needsMaintenance()) return;
		const maintenance = this.maintenance;
		if (!maintenance) {
			this.buffer.maintain();
			return;
		}
		if (this.pendingMaintenance.value) return;
		let pending: IDisposable;
		let ranSynchronously = false;
		try {
			pending = maintenance.schedule(() => {
				ranSynchronously = true;
				this.pendingMaintenance.clear();
				if (!this.isDisposed) this.buffer.maintainIfNeeded();
			});
			if (!pending || typeof pending.dispose !== "function") {
				throw new TypeError("TextModel maintenance scheduler must return a disposable");
			}
		} catch {
			this.buffer.maintain();
			return;
		}
		if (ranSynchronously) {
			pending.dispose();
			return;
		}
		this.pendingMaintenance.value = pending;
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

	/** Commits flattened line text for one already-validated block transaction. */
	private commitBlockText(text: string): { readonly version: number; readonly change?: TextModelChange } {
		const previousText = this.buffer.getText();
		const nextText = normalizeTextLineEndings(text);
		let prefixLength = 0;
		const maximumPrefixLength = Math.min(previousText.length, nextText.length);
		while (prefixLength < maximumPrefixLength && previousText.charCodeAt(prefixLength) === nextText.charCodeAt(prefixLength)) prefixLength += 1;
		let suffixLength = 0;
		const maximumSuffixLength = Math.min(previousText.length - prefixLength, nextText.length - prefixLength);
		while (
			suffixLength < maximumSuffixLength &&
			previousText.charCodeAt(previousText.length - suffixLength - 1) === nextText.charCodeAt(nextText.length - suffixLength - 1)
		) suffixLength += 1;
		const result = this.commitOffsetEdits([{
			startOffset: prefixLength,
			endOffset: previousText.length - suffixLength,
			text: nextText.slice(prefixLength, nextText.length - suffixLength),
		}], { reason: TextModelChangeReason.Blocks });
		this.history.reset();
		if (result) return { version: this._version, change: result.change };
		this._version += 1;
		const change = Object.freeze<TextModelChange>({
			version: this._version,
			transactionId: this.nextTransactionId++,
			reason: TextModelChangeReason.Blocks,
			changes: Object.freeze([Object.freeze<TextModelContentChange>({
				range: TextRange.from(TextPosition.at(0, 0), this.positionAt(this.buffer.length)),
				rangeOffset: 0,
				rangeLength: this.buffer.length,
				text: previousText,
			})]),
		});
		return { version: this._version, change };
	}

	private initializeLineIds(lineIds: readonly LineId[] | undefined): readonly LineId[] {
		if (lineIds !== undefined) {
			if (!Array.isArray(lineIds) || lineIds.length !== this.buffer.lineCount) {
				throw new RangeError(`TextModel requires exactly ${this.buffer.lineCount} line ids`);
			}
			return this.validateLineIdentities(lineIds);
		}
		return Object.freeze(Array.from({ length: this.buffer.lineCount }, () => this.allocateLineId()));
	}

	private mapLineIds(prepared: readonly PreparedEdit[]): readonly LineId[] {
		const lineIds = [...this.plainLineIds!];
		for (let index = prepared.length - 1; index >= 0; index -= 1) {
			const edit = prepared[index]!;
			const removedLineCount = edit.range.end.lineIndex - edit.range.start.lineIndex;
			const insertedLineCount = countLineBreaks(edit.text);
			const insertedLineIds = Array.from({ length: insertedLineCount }, () => this.allocateLineId());
			lineIds.splice(edit.range.start.lineIndex + 1, removedLineCount, ...insertedLineIds);
		}
		if (lineIds.length !== this.buffer.lineCount) throw new Error("TextModel line identity mapping diverged from the TextBuffer");
		return Object.freeze(lineIds);
	}

	private validateCommittedLineIds(lineIds: readonly LineId[]): readonly LineId[] {
		if (lineIds.length !== this.buffer.lineCount) {
			throw new RangeError(`Committed TextModel state requires exactly ${this.buffer.lineCount} line ids`);
		}
		return this.validateLineIdentities(lineIds);
	}

	private validateLineIdentities(lineIds: readonly LineId[]): readonly LineId[] {
		const unique = new Set<LineId>();
		for (const lineId of lineIds) {
			if (typeof lineId !== "string" || lineId.trim().length === 0) throw new TypeError("TextModel line ids must be non-empty strings");
			if (unique.has(lineId)) throw new TypeError(`Duplicate TextModel line id '${lineId}'`);
			unique.add(lineId);
			this.issuedLineIds.add(lineId);
		}
		return Object.freeze([...lineIds]);
	}

	private allocateLineId(): LineId {
		for (let attempt = 0; attempt < 1_000; attempt += 1) {
			const lineId = this.lineIdGenerator();
			if (typeof lineId !== "string" || lineId.trim().length === 0) {
				throw new TypeError("TextModel lineIdGenerator must return a non-empty string");
			}
			if (this.issuedLineIds.has(lineId)) continue;
			this.issuedLineIds.add(lineId);
			return lineId;
		}
		throw new Error("TextModel lineIdGenerator did not produce a unique identity");
	}

	private requirePlainLineSnapshot(): LineDocumentSnapshot {
		const existing = this.plainLineSnapshot;
		if (existing) return existing;
		const snapshot = this.createPlainLineSnapshot(this.lineMetadata);
		this.plainLineSnapshot = snapshot;
		return snapshot;
	}

	private createPlainLineSnapshot(metadata: LineSemanticAttributes | undefined): LineDocumentSnapshot {
		const lineIds = this.plainLineIds;
		if (!lineIds || lineIds.length !== this.buffer.lineCount) throw new Error("TextModel plain line identities are unavailable");
		return createLineDocumentSnapshot({
			lines: lineIds.map((id, lineIndex) => ({ id, text: this.buffer.getLineContent(lineIndex) })),
			metadata,
		});
	}

	private requireBlockState(): TextModelBlockState {
		this.assertNotDisposed();
		const blockState = this.blockState;
		if (!blockState) throw new ReferenceError("TextModel has no schema-backed Block state");
		return blockState;
	}

	private ensureDirectTextMutationAllowed(): void {
		if (this.blockState) throw new Error("TextModel edits must update schema-backed Blocks through dispatch()");
	}

	protected override disposeCore(): void {
		this.willDisposeEmitter.fire();
		super.disposeCore();
	}

}

function compareOffsetEdits(left: OffsetEdit, right: OffsetEdit): number {
	return left.startOffset - right.startOffset ||
		left.endOffset - right.endOffset;
}

function countLineBreaks(text: string): number {
	let count = 0;
	for (let index = text.indexOf("\n"); index >= 0; index = text.indexOf("\n", index + 1)) count += 1;
	return count;
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
