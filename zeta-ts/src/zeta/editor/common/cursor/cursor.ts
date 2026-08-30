import { Emitter, type Event } from "../../../base/common/event.js";
import { IME } from "../../../base/common/ime.js";
import { Disposable, toDisposable } from '../../../base/common/lifecycle.js';
import { CursorCollection } from './cursorCollection.js';
import { Selection } from '../core/selection.js';
import { Range } from '../core/range.js';
import { TrackedRangeStickiness } from '../model.js';
import { type TrackedRange } from '../model/trackedRange.js';
import { normalizeTextLineEndings, TextModelChangeReason, type TextModelChange } from '../core/textChange.js';
import { TextEditHistoryMergeMode } from '../core/editOperation.js';
import { TextModel } from "../model/textModel.js";
import { EditorCommandHistoryMode, type EditorEditCommand, type TextSelectionOffsets } from "../commands/editorEditCommand.js";
import { CursorChangeReason } from '../cursorEvents.js';
import { UndoRedoGroup } from '../../../platform/undoRedo/common/undoRedo.js';

export interface CursorSelectionChange {
	readonly selections: readonly Selection[];
	readonly reason: CursorChangeReason;
	readonly modelVersion: number;
}

export interface CompositionUpdate {
	readonly text: string;
	readonly selection: TextSelectionOffsets;
}

interface CursorsControllerOptions {
	readonly selectionHistoryLimit?: number;
	readonly cursorHistoryLimit?: number;
	readonly readOnly?: boolean;
}

class CursorControllerContext {
	readonly selectionHistoryLimit: number;
	readonly cursorHistoryLimit: number;
	readonly readOnly: boolean;

	constructor(readonly model: TextModel, options: CursorsControllerOptions) {
		this.selectionHistoryLimit = readLimit(options.selectionHistoryLimit, 1_000, 'selectionHistoryLimit');
		this.cursorHistoryLimit = readLimit(options.cursorHistoryLimit, 100, 'cursorHistoryLimit');
		if (options.readOnly !== undefined && typeof options.readOnly !== 'boolean') throw new TypeError('Editor read-only mode must be boolean');
		this.readOnly = options.readOnly ?? false;
	}
}

function readLimit(value: number | undefined, fallback: number, name: string): number {
	const limit = value ?? fallback;
	if (!Number.isSafeInteger(limit) || limit < 0) throw new RangeError(`${name} must be a non-negative safe integer`);
	return limit;
}

interface CompositionHost {
	isActive(): boolean;
	assertActive(): void;
	apply(command: EditorEditCommand): TextModelChange | undefined;
	commit(): void;
	cancel(): TextModelChange | undefined;
}

interface SelectionHistoryEntry {
	readonly before: readonly Selection[];
	readonly after: readonly Selection[];
}

interface ActiveComposition {
	readonly historyGroup: UndoRedoGroup;
	transactionId?: number;
	valid: boolean;
}

interface AutoClosedAction {
	readonly open: string;
	readonly close: string;
	readonly enclosing: TrackedRange;
	readonly closer: TrackedRange;
}

/**
 * Per-editor selection state for one shared `TextModel`.
 *
 * Text remains document-owned. This controller owns only one editor instance's
 * tracked selections and command-level selection history.
 */
export class CursorsController extends Disposable {
	private readonly context: CursorControllerContext;
	private readonly changeEmitter =
		this._register(new Emitter<CursorSelectionChange>());
	private readonly cursors: CursorCollection;
	private readonly selectionHistory =
		new Map<number, SelectionHistoryEntry>();
	private readonly selectionHistoryOrder: number[] = [];
	private readonly cursorHistory: Array<readonly Selection[]> = [];
	private autoClosedActions: AutoClosedAction[] = [];
	private currentSelections: readonly Selection[];
	private activeHistoryGroup: UndoRedoGroup | undefined;
	private activeHistoryMode: EditorCommandHistoryMode | undefined;
	private activeComposition: ActiveComposition | undefined;
	private executingCommand = false;

	readonly onDidChange: Event<CursorSelectionChange> =
		this.changeEmitter.event;

	constructor(
		private readonly model: TextModel,
		initialSelections: readonly Selection[],
		options: CursorsControllerOptions = {},
	) {
		super();
		this.context = new CursorControllerContext(model, options);
		this.currentSelections = initialSelections;
		this.cursors = this._register(new CursorCollection(model, initialSelections));
		try {
			this.installSelections(initialSelections);
			this._register(model.onDidChangeContent(change => this.acceptModelChange(change)));
			this._register(toDisposable(() => {
				this.selectionHistory.clear();
				this.selectionHistoryOrder.length = 0;
				this.cursorHistory.length = 0;
				for (const action of this.autoClosedActions) this.disposeAutoClosedAction(action);
				this.autoClosedActions = [];
				if (this.activeComposition) {
					this.activeComposition.valid = false;
					this.activeComposition = undefined;
				}
			}));
		} catch (error) {
			this.dispose();
			throw error;
		}
	}

	get selections(): readonly Selection[] {
		this.assertNotDisposed();
		return this.currentSelections;
	}

	get textModel(): TextModel {
		this.assertNotDisposed();
		return this.model;
	}

	/** Returns closing ranges that are still owned by this cursor controller. */
	getAutoClosedCharacters(): readonly Range[] {
		this.assertNotDisposed();
		this.pruneAutoClosedActions();
		return Object.freeze(this.autoClosedActions.map(action => action.closer.range));
	}

	/** Records the paired ranges produced by one committed type operation. */
	recordAutoClosedCharacters(closers: readonly Range[], enclosing: readonly Range[], committedModelVersion: number): void {
		this.assertNotDisposed();
		if (!Array.isArray(closers) || !Array.isArray(enclosing) || closers.length !== enclosing.length) {
			throw new RangeError('Auto-closed character and enclosing ranges must have the same length');
		}
		if (!Number.isSafeInteger(committedModelVersion) || committedModelVersion < 1) {
			throw new RangeError('Committed model version must be a positive safe integer');
		}
		if (committedModelVersion !== this.model.version) return;
		const additions: AutoClosedAction[] = [];
		try {
			for (let index = 0; index < closers.length; index += 1) {
				additions.push(this.createAutoClosedAction(closers[index]!, enclosing[index]!));
			}
		} catch (error) {
			for (const action of additions) this.disposeAutoClosedAction(action);
			throw error;
		}
		this.autoClosedActions.push(...additions);
		this.pruneAutoClosedActions();
	}

	/** Whether this editor instance may submit document-changing commands. */
	get readOnly(): boolean {
		this.assertNotDisposed();
		return this.context.readOnly;
	}

	setSelections(selections: readonly Selection[]): void {
		this.assertNotDisposed();
		this.assertNoActiveComposition("set selections");
		this.breakHistoryGroup();
		this.cursorHistory.length = 0;
		this.installSelections(
			selections,
			CursorChangeReason.NotSet,
		);
	}

	/** Records one cursor-only selection transition that `undoCursorOperation` may restore. */
	setCursorSelections(selections: readonly Selection[]): void {
		this.assertNotDisposed();
		this.assertNoActiveComposition("set cursor selections");
		this.breakHistoryGroup();
		if (CursorCollection.selectionsEqual(this.currentSelections, selections)) return;
		this.rememberCursorSelections(this.currentSelections);
		this.installSelections(selections, CursorChangeReason.Explicit);
	}

	/** Restores the preceding cursor-only selection state without changing document undo history. */
	undoCursorOperation(): boolean {
		this.assertNotDisposed();
		this.assertNoActiveComposition("undo cursor operation");
		this.breakHistoryGroup();
		const previous = this.cursorHistory.pop();
		if (!previous) return false;
		this.installSelections(previous, CursorChangeReason.Explicit);
		return true;
	}

	/** Ends command coalescing without creating an empty history entry. */
	pushUndoStop(): void {
		this.assertNotDisposed();
		this.assertNoActiveComposition("push an undo stop");
		this.breakHistoryGroup();
	}

	execute(command: EditorEditCommand): TextModelChange | undefined {
		this.assertNotDisposed();
		this.assertNoActiveComposition("execute a command");
		if (this.context.readOnly) return undefined;
		this.cursorHistory.length = 0;
		const historyGroup = this.historyGroupFor(command.historyMode);
		return this.executeCommand(
			command,
			historyGroup,
			TextEditHistoryMergeMode.Sequential,
		);
	}

	beginComposition(): CompositionSession {
		this.assertNotDisposed();
		this.assertNoActiveComposition("begin another composition");
		if (this.context.readOnly) throw new Error("Cannot begin composition in a read-only editor");
		if (!IME.enabled) {
			throw new Error("IME composition is currently disabled");
		}
		if (this.currentSelections.length !== 1) {
			throw new Error(
				"IME composition currently requires exactly one selection",
			);
		}
		this.breakHistoryGroup();
		this.cursorHistory.length = 0;
		const initialSelections = this.currentSelections;
		const initialRange = initialSelections[0]!;
		const startOffset = this.model.offsetAt(initialRange.getStartPosition());
		const endOffset = this.model.offsetAt(initialRange.getEndPosition());
		const state: ActiveComposition = {
			historyGroup: new UndoRedoGroup(),
			valid: true,
		};
		this.model.beginHistoryRevision(state.historyGroup);
		this.activeComposition = state;
		return new CompositionSession(
			this.model,
			startOffset,
			endOffset,
			{
				isActive: () =>
					state.valid && this.activeComposition === state,
				assertActive: () => this.assertActiveComposition(state),
				apply: command => {
					const change = this.executeCommand(
						command,
						state.historyGroup,
						TextEditHistoryMergeMode.ReplacePrevious,
					);
					if (change) state.transactionId = change.transactionId;
					return change;
				},
				commit: () => {
					this.assertActiveComposition(state);
					state.valid = false;
					this.activeComposition = undefined;
					const retained = this.model.finishHistoryRevision(
						state.historyGroup,
					);
					if (!retained && state.transactionId !== undefined) {
						this.forgetSelectionHistory(state.transactionId);
					}
				},
				cancel: () => {
					this.assertActiveComposition(state);
					state.valid = false;
					this.activeComposition = undefined;
					const change = this.model.cancelHistoryRevision(
						state.historyGroup,
					);
					if (!change) {
						if (state.transactionId !== undefined) {
							this.forgetSelectionHistory(state.transactionId);
						}
						this.installSelections(
							initialSelections,
							CursorChangeReason.Undo,
						);
					}
					return change;
				},
			},
		);
	}

	private executeCommand(
		command: EditorEditCommand,
		historyGroup: UndoRedoGroup | undefined,
		historyMergeMode: TextEditHistoryMergeMode,
	): TextModelChange | undefined {
		const resultLength = CursorCollection.calculateResultLength(this.model, command.edits);
		CursorCollection.validateSelectionOffsets(
			command.selectionsAfter,
			command.primarySelectionIndex,
			resultLength,
		);
		const before = this.currentSelections;
		const versionBefore = this.model.version;
		let change: TextModelChange | undefined;
		this.executingCommand = true;
		try {
			change = this.model.applyOperations(
				command.edits,
				historyGroup
					? { historyGroup, historyMergeMode }
					: {},
			);
		} catch (error) {
			this.breakHistoryGroup();
			throw error;
		} finally {
			this.executingCommand = false;
		}

		if (change && this.model.version !== change.version) {
			this.breakHistoryGroup();
			this.invalidateActiveComposition();
			this.refreshTrackedSelections(
				CursorChangeReason.RecoverFromMarkers,
				true,
				true,
			);
			return change;
		}
		if (!change && this.model.version !== versionBefore) {
			this.breakHistoryGroup();
			this.invalidateActiveComposition();
			this.refreshTrackedSelections(
				CursorChangeReason.RecoverFromMarkers,
			);
			return undefined;
		}
		if (
			!change &&
			historyMergeMode !== TextEditHistoryMergeMode.ReplacePrevious
		) {
			this.breakHistoryGroup();
		}

		const after = CursorCollection.selectionsFromOffsets(
			this.model,
			command.selectionsAfter,
			command.primarySelectionIndex,
		);
		this.installSelections(
			after,
			CursorChangeReason.NotSet,
		);
		if (change) {
			this.rememberSelectionHistory(
				change.transactionId,
				{ before, after },
			);
		}
		return change;
	}

	undo(): TextModelChange | undefined {
		this.assertNotDisposed();
		this.assertNoActiveComposition("undo");
		if (this.context.readOnly) return undefined;
		this.cursorHistory.length = 0;
		this.breakHistoryGroup();
		return this.model.undo();
	}

	redo(): TextModelChange | undefined {
		this.assertNotDisposed();
		this.assertNoActiveComposition("redo");
		if (this.context.readOnly) return undefined;
		this.cursorHistory.length = 0;
		this.breakHistoryGroup();
		return this.model.redo();
	}

	private acceptModelChange(change: TextModelChange): void {
		if (this.executingCommand) {
			this.refreshTrackedSelections(
				CursorChangeReason.RecoverFromMarkers,
				false,
			);
			return;
		}
		this.breakHistoryGroup();
		this.cursorHistory.length = 0;
		this.invalidateActiveComposition();
		if (change.reason === TextModelChangeReason.Reset) {
			this.selectionHistory.clear();
			this.selectionHistoryOrder.length = 0;
		}
		const history = this.selectionHistory.get(change.transactionId);
		if (history && change.reason === TextModelChangeReason.Undo) {
			this.installSelections(
				history.before,
				CursorChangeReason.Undo,
			);
			return;
		}
		if (history && change.reason === TextModelChangeReason.Redo) {
			this.installSelections(
				history.after,
				CursorChangeReason.Redo,
			);
			return;
		}
		if (
			history &&
			change.reason === TextModelChangeReason.HistoryCancellation
		) {
			this.installSelections(
				history.before,
				CursorChangeReason.Undo,
			);
			this.forgetSelectionHistory(change.transactionId);
			return;
		}
		this.refreshTrackedSelections(
			change.reason === TextModelChangeReason.Reset
				? CursorChangeReason.ContentFlush
				: CursorChangeReason.RecoverFromMarkers,
		);
	}

	private installSelections(
		selections: readonly Selection[],
		reason?: CursorChangeReason,
	): void {
		CursorCollection.validateSelections(this.model, selections);
		const previous = this.currentSelections;
		this.cursors.setSelections(selections);
		this.currentSelections = selections;
		this.pruneAutoClosedActions();
		if (reason !== undefined && !CursorCollection.selectionsEqual(previous, selections)) {
			this.changeEmitter.fire(Object.freeze({
				selections,
				reason,
				modelVersion: this.model.version,
			}));
		}
	}

	private refreshTrackedSelections(
		reason: CursorChangeReason,
		notify = true,
		forceNotify = false,
	): void {
		const selections = this.cursors.getSelections();
		const previous = this.currentSelections;
		this.currentSelections = selections;
		this.pruneAutoClosedActions();
		if (notify && (forceNotify || !CursorCollection.selectionsEqual(previous, selections))) {
			this.changeEmitter.fire(Object.freeze({
				selections,
				reason,
				modelVersion: this.model.version,
			}));
		}
	}

	private rememberSelectionHistory(
		transactionId: number,
		entry: SelectionHistoryEntry,
	): void {
		const previous = this.selectionHistory.get(transactionId);
		if (previous) {
			this.selectionHistory.set(transactionId, {
				before: previous.before,
				after: entry.after,
			});
			return;
		}
		this.selectionHistory.set(transactionId, entry);
		this.selectionHistoryOrder.push(transactionId);
		while (
			this.selectionHistoryOrder.length >
			this.context.selectionHistoryLimit
		) {
			const oldest = this.selectionHistoryOrder.shift();
			if (oldest !== undefined) this.selectionHistory.delete(oldest);
		}
	}

	private rememberCursorSelections(selections: readonly Selection[]): void {
		if (this.context.cursorHistoryLimit === 0) return;
		this.cursorHistory.push(selections);
		while (this.cursorHistory.length > this.context.cursorHistoryLimit) this.cursorHistory.shift();
	}

	private forgetSelectionHistory(transactionId: number): void {
		this.selectionHistory.delete(transactionId);
		const index = this.selectionHistoryOrder.indexOf(transactionId);
		if (index >= 0) this.selectionHistoryOrder.splice(index, 1);
	}

	private historyGroupFor(
		mode: EditorCommandHistoryMode | undefined,
	): UndoRedoGroup | undefined {
		if (
			mode === undefined ||
			mode === EditorCommandHistoryMode.Isolated
		) {
			this.breakHistoryGroup();
			return undefined;
		}
		if (mode === EditorCommandHistoryMode.BeginCoalescedTyping) {
			this.breakHistoryGroup();
			this.activeHistoryGroup = new UndoRedoGroup();
			this.activeHistoryMode = EditorCommandHistoryMode.CoalesceTyping;
			return this.activeHistoryGroup;
		}
		if (
			mode !== EditorCommandHistoryMode.CoalesceTyping &&
			mode !== EditorCommandHistoryMode.CoalesceBackspace &&
			mode !== EditorCommandHistoryMode.CoalesceDelete
		) {
			throw new TypeError("Unknown editor command history mode");
		}
		if (!this.activeHistoryGroup || this.activeHistoryMode !== mode) {
			this.activeHistoryGroup = new UndoRedoGroup();
			this.activeHistoryMode = mode;
		}
		return this.activeHistoryGroup;
	}

	private breakHistoryGroup(): void {
		this.activeHistoryGroup = undefined;
		this.activeHistoryMode = undefined;
	}

	private assertNoActiveComposition(operation: string): void {
		if (this.activeComposition?.valid) {
			throw new Error(`Cannot ${operation} during an active composition`);
		}
	}

	private assertActiveComposition(state: ActiveComposition): void {
		this.assertNotDisposed();
		if (!state.valid || this.activeComposition !== state) {
			throw new Error("Editor composition is no longer active");
		}
	}

	private invalidateActiveComposition(): void {
		if (!this.activeComposition) return;
		this.activeComposition.valid = false;
		this.activeComposition = undefined;
	}

	private createAutoClosedAction(closerRange: Range, enclosingRange: Range): AutoClosedAction {
		const enclosingStart = this.model.offsetAt(enclosingRange.getStartPosition());
		const enclosingEnd = this.model.offsetAt(enclosingRange.getEndPosition());
		const closeStart = this.model.offsetAt(closerRange.getStartPosition());
		const closeEnd = this.model.offsetAt(closerRange.getEndPosition());
		if (enclosingStart >= closeStart || closeEnd !== enclosingEnd) {
			throw new RangeError('Auto-closed ranges must describe an opener followed by its closer');
		}
		const open = this.model.getTextInRange(Range.fromPositions(enclosingRange.getStartPosition(), closerRange.getStartPosition()));
		const close = this.model.getTextInRange(closerRange);
		if (open.length === 0 || close.length === 0) throw new RangeError('Auto-closed ranges must contain text');
		const enclosing = this.model.trackRange(enclosingRange, TrackedRangeStickiness.NeverGrowsWhenTypingAtEdges);
		try {
			const closer = this.model.trackRange(closerRange, TrackedRangeStickiness.NeverGrowsWhenTypingAtEdges);
			return { open, close, enclosing, closer };
		} catch (error) {
			enclosing.dispose();
			throw error;
		}
	}

	private pruneAutoClosedActions(): void {
		if (this.isDisposed || this.autoClosedActions.length === 0) return;
		const retained: AutoClosedAction[] = [];
		for (const action of this.autoClosedActions) {
			if (this.isAutoClosedActionValid(action)) retained.push(action);
			else this.disposeAutoClosedAction(action);
		}
		this.autoClosedActions = retained;
	}

	private isAutoClosedActionValid(action: AutoClosedAction): boolean {
		const enclosing = action.enclosing.range;
		const closer = action.closer.range;
		if (enclosing.startLineNumber !== enclosing.endLineNumber || closer.startLineNumber !== closer.endLineNumber) return false;
		if (closer.endLineNumber !== enclosing.endLineNumber || closer.endColumn !== enclosing.endColumn) return false;
		if (this.model.getTextInRange(closer) !== action.close) return false;
		if (this.model.getTextInRange(Range.fromPositions(enclosing.getStartPosition(), closer.getStartPosition())) !== action.open) return false;
		return this.currentSelections.some(selection => (
			enclosing.getStartPosition().isBefore(selection.getStartPosition()) &&
			selection.getEndPosition().isBefore(enclosing.getEndPosition())
		));
	}

	private disposeAutoClosedAction(action: AutoClosedAction): void {
		action.closer.dispose();
		action.enclosing.dispose();
	}

}

export class CompositionSession {
	private currentEndOffset: number;
	private closed = false;

	constructor(
		private readonly model: TextModel,
		private readonly startOffset: number,
		endOffset: number,
		private readonly host: CompositionHost,
	) {
		this.currentEndOffset = endOffset;
	}

	public get active(): boolean {
		return !this.closed && this.host.isActive();
	}

	public get currentRange(): Range {
		this.ensureActive();
		this.host.assertActive();
		return Range.fromPositions(this.model.positionAt(this.startOffset), this.model.positionAt(this.currentEndOffset));
	}

	public update(update: CompositionUpdate): TextModelChange | undefined {
		this.ensureActive();
		this.host.assertActive();
		if (typeof update.text !== 'string') throw new TypeError('CompositionUpdate.text must be a string');
		const text = normalizeTextLineEndings(update.text);
		validateRelativeSelection(update.selection, text.length);
		const change = this.host.apply({
			edits: [{
				range: Range.fromPositions(this.model.positionAt(this.startOffset), this.model.positionAt(this.currentEndOffset)),
				text,
			}],
			selectionsAfter: [{
				anchorOffset: this.startOffset + update.selection.anchorOffset,
				activeOffset: this.startOffset + update.selection.activeOffset,
			}],
			primarySelectionIndex: 0,
		});
		this.host.assertActive();
		this.currentEndOffset = this.startOffset + text.length;
		return change;
	}

	public commit(): void {
		this.ensureActive();
		this.host.assertActive();
		this.closed = true;
		this.host.commit();
	}

	public cancel(): TextModelChange | undefined {
		this.ensureActive();
		this.host.assertActive();
		this.closed = true;
		return this.host.cancel();
	}

	private ensureActive(): void {
		if (this.closed) throw new ReferenceError('Editor composition is already closed');
	}
}

function validateRelativeSelection(selection: TextSelectionOffsets, textLength: number): void {
	assertRelativeOffset(selection.anchorOffset, textLength, 'selection.anchorOffset');
	assertRelativeOffset(selection.activeOffset, textLength, 'selection.activeOffset');
}

function assertRelativeOffset(offset: number, textLength: number, name: string): void {
	if (!Number.isSafeInteger(offset) || offset < 0 || offset > textLength) {
		throw new RangeError(`${name} must be between 0 and ${textLength}`);
	}
}
