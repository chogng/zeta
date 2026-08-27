import { Emitter, type Event } from "../../../base/common/event.js";
import { IME } from "../../../base/common/ime.js";
import { Disposable, DisposableStore, toDisposable } from "../../../base/common/lifecycle.js";
import { EditorCompositionSession } from "./editorComposition.js";
import { calculateResultLength, readSelectionHistoryLimit, selectionSetFromOffsets, selectionSetsEqual, validateSelectionOffsets, validateSelectionSet } from "./editorSelectionOperations.js";
import { SelectionDirection, TextSelection, TextSelectionSet } from "../core/selection.js";
import { TextEditHistoryGroup, TextEditHistoryMergeMode, TextModelChangeReason, type TextModelChange } from "../core/text.js";
import { TextModel } from "../model/textModel.js";
import { TrackedRangeStickiness, type TrackedRange } from "../model/trackedRange.js";
import { EditorCommandHistoryMode, type EditorEditCommand, type TextSelectionOffsets } from "../commands/editorEditCommand.js";

export { EditorCommandHistoryMode } from "../commands/editorEditCommand.js";
export type { EditorEditCommand, TextSelectionOffsets } from "../commands/editorEditCommand.js";

export interface EditorSelectionControllerOptions {
	readonly selectionHistoryLimit?: number;
	/** Limits reversible cursor-only operations independently from text undo history. */
	readonly cursorHistoryLimit?: number;
	/** Prevents this editor instance from committing text commands while preserving navigation and selection. */
	readonly readOnly?: boolean;
}

export enum EditorSelectionChangeReason {
	Explicit = "explicit",
	Command = "command",
	Undo = "undo",
	Redo = "redo",
	ModelChange = "modelChange",
	HistoryCancellation = "historyCancellation",
	CursorOperation = "cursorOperation",
	CursorUndo = "cursorUndo",
}

export interface EditorSelectionChange {
	readonly selections: TextSelectionSet;
	readonly reason: EditorSelectionChangeReason;
	readonly modelVersion: number;
}

interface SelectionHistoryEntry {
	readonly before: TextSelectionSet;
	readonly after: TextSelectionSet;
}

interface TrackedSelection {
	readonly range: TrackedRange;
	readonly direction: SelectionDirection;
}

interface ActiveComposition {
	readonly historyGroup: TextEditHistoryGroup;
	transactionId?: number;
	valid: boolean;
}

const DEFAULT_CURSOR_HISTORY_LIMIT = 100;

function readReadOnly(value: boolean | undefined): boolean {
	if (value !== undefined && typeof value !== "boolean") throw new TypeError("Editor read-only mode must be boolean");
	return value ?? false;
}

function readCursorHistoryLimit(value: number | undefined): number {
	const limit = value ?? DEFAULT_CURSOR_HISTORY_LIMIT;
	if (!Number.isSafeInteger(limit) || limit < 0) throw new RangeError("Cursor history limit must be a non-negative safe integer");
	return limit;
}

/**
 * Per-editor selection state for one shared `TextModel`.
 *
 * Text remains document-owned. This controller owns only one editor instance's
 * tracked selections and command-level selection history.
 */
export class EditorSelectionController extends Disposable {
	private readonly changeEmitter =
		this._register(new Emitter<EditorSelectionChange>());
	private readonly trackedSelectionResources =
		this._register(new DisposableStore());
	private readonly selectionHistory =
		new Map<number, SelectionHistoryEntry>();
	private readonly selectionHistoryOrder: number[] = [];
	private readonly selectionHistoryLimit: number;
	private readonly cursorHistory: TextSelectionSet[] = [];
	private readonly cursorHistoryLimit: number;
	private readonly readOnlyMode: boolean;
	private trackedSelections: TrackedSelection[] = [];
	private currentSelections: TextSelectionSet;
	private activeHistoryGroup: TextEditHistoryGroup | undefined;
	private activeHistoryMode: EditorCommandHistoryMode | undefined;
	private activeComposition: ActiveComposition | undefined;
	private executingCommand = false;

	readonly onDidChange: Event<EditorSelectionChange> =
		this.changeEmitter.event;

	constructor(
		private readonly model: TextModel,
		initialSelections: TextSelectionSet,
		options: EditorSelectionControllerOptions = {},
	) {
		super();
		this.selectionHistoryLimit = readSelectionHistoryLimit(
			options.selectionHistoryLimit,
		);
		this.cursorHistoryLimit = readCursorHistoryLimit(options.cursorHistoryLimit);
		this.readOnlyMode = readReadOnly(options.readOnly);
		this.currentSelections = initialSelections;
		try {
			this.installSelections(initialSelections);
			this._register(model.onDidChange(change => this.acceptModelChange(change)));
			this._register(toDisposable(() => {
				this.trackedSelections = [];
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
		return this.readOnlyMode;
	}

	setSelections(selections: TextSelectionSet): void {
		this.assertNotDisposed();
		this.assertNoActiveComposition("set selections");
		this.breakHistoryGroup();
		this.cursorHistory.length = 0;
		this.installSelections(
			selections,
			EditorSelectionChangeReason.Explicit,
		);
	}

	/** Records one cursor-only selection transition that `undoCursorOperation` may restore. */
	setCursorSelections(selections: TextSelectionSet): void {
		this.assertNotDisposed();
		this.assertNoActiveComposition("set cursor selections");
		this.breakHistoryGroup();
		if (selectionSetsEqual(this.currentSelections, selections)) return;
		this.rememberCursorSelections(this.currentSelections);
		this.installSelections(selections, EditorSelectionChangeReason.CursorOperation);
	}

	/** Restores the preceding cursor-only selection state without changing document undo history. */
	undoCursorOperation(): boolean {
		this.assertNotDisposed();
		this.assertNoActiveComposition("undo cursor operation");
		this.breakHistoryGroup();
		const previous = this.cursorHistory.pop();
		if (!previous) return false;
		this.installSelections(previous, EditorSelectionChangeReason.CursorUndo);
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
		if (this.readOnlyMode) return undefined;
		this.cursorHistory.length = 0;
		const historyGroup = this.historyGroupFor(command.historyMode);
		return this.executeCommand(
			command,
			historyGroup,
			TextEditHistoryMergeMode.Sequential,
		);
	}

	beginComposition(): EditorCompositionSession {
		this.assertNotDisposed();
		this.assertNoActiveComposition("begin another composition");
		if (this.readOnlyMode) throw new Error("Cannot begin composition in a read-only editor");
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
		return new EditorCompositionSession(
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
							EditorSelectionChangeReason.HistoryCancellation,
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
		const resultLength = calculateResultLength(this.model, command.edits);
		validateSelectionOffsets(
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
				EditorSelectionChangeReason.ModelChange,
				true,
				true,
			);
			return change;
		}
		if (!change && this.model.version !== versionBefore) {
			this.breakHistoryGroup();
			this.invalidateActiveComposition();
			this.refreshTrackedSelections(
				EditorSelectionChangeReason.ModelChange,
			);
			return undefined;
		}
		if (
			!change &&
			historyMergeMode !== TextEditHistoryMergeMode.ReplacePrevious
		) {
			this.breakHistoryGroup();
		}

		const after = selectionSetFromOffsets(
			this.model,
			command.selectionsAfter,
			command.primarySelectionIndex,
		);
		this.installSelections(
			after,
			EditorSelectionChangeReason.Command,
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
		if (this.readOnlyMode) return undefined;
		this.cursorHistory.length = 0;
		this.breakHistoryGroup();
		return this.model.undo();
	}

	redo(): TextModelChange | undefined {
		this.assertNotDisposed();
		this.assertNoActiveComposition("redo");
		if (this.readOnlyMode) return undefined;
		this.cursorHistory.length = 0;
		this.breakHistoryGroup();
		return this.model.redo();
	}

	private acceptModelChange(change: TextModelChange): void {
		if (this.executingCommand) {
			this.refreshTrackedSelections(
				EditorSelectionChangeReason.ModelChange,
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
				EditorSelectionChangeReason.Undo,
			);
			return;
		}
		if (history && change.reason === TextModelChangeReason.Redo) {
			this.installSelections(
				history.after,
				EditorSelectionChangeReason.Redo,
			);
			return;
		}
		if (
			history &&
			change.reason === TextModelChangeReason.HistoryCancellation
		) {
			this.installSelections(
				history.before,
				EditorSelectionChangeReason.HistoryCancellation,
			);
			this.forgetSelectionHistory(change.transactionId);
			return;
		}
		this.refreshTrackedSelections(
			EditorSelectionChangeReason.ModelChange,
		);
	}

	private installSelections(
		selections: TextSelectionSet,
		reason?: EditorSelectionChangeReason,
	): void {
		validateSelectionSet(this.model, selections);
		const previous = this.currentSelections;
		this.trackedSelectionResources.clear();
		this.trackedSelections = selections.selections.map(selection => {
			const range = this.model.trackRange(
				selection.range,
				TrackedRangeStickiness.NeverGrowsAtEdges,
			);
			this.trackedSelectionResources.add(toDisposable(() => range.dispose()));
			return { range, direction: selection.direction };
		});
		this.currentSelections = selections;
		if (reason && !selectionSetsEqual(previous, selections)) {
			this.changeEmitter.fire(Object.freeze({
				selections,
				reason,
				modelVersion: this.model.version,
			}));
		}
	}

	private refreshTrackedSelections(
		reason: EditorSelectionChangeReason,
		notify = true,
		forceNotify = false,
	): void {
		const selections = TextSelectionSet.withPrimary(
			this.trackedSelections.map(tracked => {
				const range = tracked.range.range;
				return tracked.direction === SelectionDirection.Backward
					? TextSelection.from(range.end, range.start)
					: TextSelection.from(range.start, range.end);
			}),
			this.currentSelections.primaryIndex,
		);
		const previous = this.currentSelections;
		this.currentSelections = selections;
		if (notify && (forceNotify || !selectionSetsEqual(previous, selections))) {
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
			this.selectionHistoryLimit
		) {
			const oldest = this.selectionHistoryOrder.shift();
			if (oldest !== undefined) this.selectionHistory.delete(oldest);
		}
	}

	private rememberCursorSelections(selections: TextSelectionSet): void {
		if (this.cursorHistoryLimit === 0) return;
		this.cursorHistory.push(selections);
		while (this.cursorHistory.length > this.cursorHistoryLimit) this.cursorHistory.shift();
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
