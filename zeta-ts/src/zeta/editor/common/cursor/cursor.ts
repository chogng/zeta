import { Emitter, type Event } from "../../../base/common/event.js";
import { IME } from "../../../base/common/ime.js";
import { Disposable, toDisposable } from '../../../base/common/lifecycle.js';
import { CursorCollection } from './cursorCollection.js';
import { CursorContext } from './cursorContext.js';
import { TextSelectionSet } from '../core/selection.js';
import { normalizeTextLineEndings, TextEditHistoryGroup, TextEditHistoryMergeMode, TextModelChangeReason, TextRange, type TextModelChange } from '../core/text.js';
import { TextModel } from "../model/textModel.js";
import { EditorCommandHistoryMode, type EditorEditCommand, type TextSelectionOffsets } from "../commands/editorEditCommand.js";

export enum CursorChangeReason {
	Explicit = "explicit",
	Command = "command",
	Undo = "undo",
	Redo = "redo",
	ModelChange = "modelChange",
	HistoryCancellation = "historyCancellation",
	CursorOperation = "cursorOperation",
	CursorUndo = "cursorUndo",
}

export interface CursorStateChangedEvent {
	readonly selections: TextSelectionSet;
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

interface CompositionHost {
	isActive(): boolean;
	assertActive(): void;
	apply(command: EditorEditCommand): TextModelChange | undefined;
	commit(): void;
	cancel(): TextModelChange | undefined;
}

interface SelectionHistoryEntry {
	readonly before: TextSelectionSet;
	readonly after: TextSelectionSet;
}

interface ActiveComposition {
	readonly historyGroup: TextEditHistoryGroup;
	transactionId?: number;
	valid: boolean;
}

/**
 * Per-editor selection state for one shared `TextModel`.
 *
 * Text remains document-owned. This controller owns only one editor instance's
 * tracked selections and command-level selection history.
 */
export class CursorsController extends Disposable {
	private readonly context: CursorContext;
	private readonly changeEmitter =
		this._register(new Emitter<CursorStateChangedEvent>());
	private readonly cursors: CursorCollection;
	private readonly selectionHistory =
		new Map<number, SelectionHistoryEntry>();
	private readonly selectionHistoryOrder: number[] = [];
	private readonly cursorHistory: TextSelectionSet[] = [];
	private currentSelections: TextSelectionSet;
	private activeHistoryGroup: TextEditHistoryGroup | undefined;
	private activeHistoryMode: EditorCommandHistoryMode | undefined;
	private activeComposition: ActiveComposition | undefined;
	private executingCommand = false;

	readonly onDidChange: Event<CursorStateChangedEvent> =
		this.changeEmitter.event;

	constructor(
		private readonly model: TextModel,
		initialSelections: TextSelectionSet,
		options: CursorsControllerOptions = {},
	) {
		super();
		this.context = new CursorContext(model, options);
		this.currentSelections = initialSelections;
		this.cursors = this._register(new CursorCollection(model, initialSelections));
		try {
			this.installSelections(initialSelections);
			this._register(model.onDidChange(change => this.acceptModelChange(change)));
			this._register(toDisposable(() => {
				this.selectionHistory.clear();
				this.selectionHistoryOrder.length = 0;
				this.cursorHistory.length = 0;
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

	get selections(): TextSelectionSet {
		this.assertNotDisposed();
		return this.currentSelections;
	}

	get textModel(): TextModel {
		this.assertNotDisposed();
		return this.model;
	}

	/** Whether this editor instance may submit document-changing commands. */
	get readOnly(): boolean {
		this.assertNotDisposed();
		return this.context.readOnly;
	}

	setSelections(selections: TextSelectionSet): void {
		this.assertNotDisposed();
		this.assertNoActiveComposition("set selections");
		this.breakHistoryGroup();
		this.cursorHistory.length = 0;
		this.installSelections(
			selections,
			CursorChangeReason.Explicit,
		);
	}

	/** Records one cursor-only selection transition that `undoCursorOperation` may restore. */
	setCursorSelections(selections: TextSelectionSet): void {
		this.assertNotDisposed();
		this.assertNoActiveComposition("set cursor selections");
		this.breakHistoryGroup();
		if (CursorCollection.selectionsEqual(this.currentSelections, selections)) return;
		this.rememberCursorSelections(this.currentSelections);
		this.installSelections(selections, CursorChangeReason.CursorOperation);
	}

	/** Restores the preceding cursor-only selection state without changing document undo history. */
	undoCursorOperation(): boolean {
		this.assertNotDisposed();
		this.assertNoActiveComposition("undo cursor operation");
		this.breakHistoryGroup();
		const previous = this.cursorHistory.pop();
		if (!previous) return false;
		this.installSelections(previous, CursorChangeReason.CursorUndo);
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
		if (this.currentSelections.selections.length !== 1) {
			throw new Error(
				"IME composition currently requires exactly one selection",
			);
		}
		this.breakHistoryGroup();
		this.cursorHistory.length = 0;
		const initialSelections = this.currentSelections;
		const initialRange = initialSelections.primary.range;
		const startOffset = this.model.offsetAt(initialRange.start);
		const endOffset = this.model.offsetAt(initialRange.end);
		const state: ActiveComposition = {
			historyGroup: TextEditHistoryGroup.create(),
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
							CursorChangeReason.HistoryCancellation,
						);
					}
					return change;
				},
			},
		);
	}

	private executeCommand(
		command: EditorEditCommand,
		historyGroup: TextEditHistoryGroup | undefined,
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
			change = this.model.applyEdits(
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
				CursorChangeReason.ModelChange,
				true,
				true,
			);
			return change;
		}
		if (!change && this.model.version !== versionBefore) {
			this.breakHistoryGroup();
			this.invalidateActiveComposition();
			this.refreshTrackedSelections(
				CursorChangeReason.ModelChange,
			);
			return undefined;
		}
		if (
			!change &&
			historyMergeMode !== TextEditHistoryMergeMode.ReplacePrevious
		) {
			this.breakHistoryGroup();
		}

		const after = CursorCollection.selectionSetFromOffsets(
			this.model,
			command.selectionsAfter,
			command.primarySelectionIndex,
		);
		this.installSelections(
			after,
			CursorChangeReason.Command,
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
				CursorChangeReason.ModelChange,
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
				CursorChangeReason.HistoryCancellation,
			);
			this.forgetSelectionHistory(change.transactionId);
			return;
		}
		this.refreshTrackedSelections(
			CursorChangeReason.ModelChange,
		);
	}

	private installSelections(
		selections: TextSelectionSet,
		reason?: CursorChangeReason,
	): void {
		CursorCollection.validateSelectionSet(this.model, selections);
		const previous = this.currentSelections;
		this.cursors.setSelections(selections);
		this.currentSelections = selections;
		if (reason && !CursorCollection.selectionsEqual(previous, selections)) {
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

	private rememberCursorSelections(selections: TextSelectionSet): void {
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
	): TextEditHistoryGroup | undefined {
		if (
			mode === undefined ||
			mode === EditorCommandHistoryMode.Isolated
		) {
			this.breakHistoryGroup();
			return undefined;
		}
		if (mode === EditorCommandHistoryMode.BeginCoalescedTyping) {
			this.breakHistoryGroup();
			this.activeHistoryGroup = TextEditHistoryGroup.create();
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
			this.activeHistoryGroup = TextEditHistoryGroup.create();
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

	public get currentRange(): TextRange {
		this.ensureActive();
		this.host.assertActive();
		return TextRange.from(this.model.positionAt(this.startOffset), this.model.positionAt(this.currentEndOffset));
	}

	public update(update: CompositionUpdate): TextModelChange | undefined {
		this.ensureActive();
		this.host.assertActive();
		if (typeof update.text !== 'string') throw new TypeError('CompositionUpdate.text must be a string');
		const text = normalizeTextLineEndings(update.text);
		validateRelativeSelection(update.selection, text.length);
		const change = this.host.apply({
			edits: [{
				range: TextRange.from(this.model.positionAt(this.startOffset), this.model.positionAt(this.currentEndOffset)),
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
