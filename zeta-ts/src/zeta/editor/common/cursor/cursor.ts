import { onUnexpectedError } from '../../../base/common/errors.js';
import { Emitter, type Event } from "../../../base/common/event.js";
import { IME } from "../../../base/common/ime.js";
import { Disposable, toDisposable } from '../../../base/common/lifecycle.js';
import { SelectionSet } from './selectionSet.js';
import { SelectionSetTracker, validateSelectionSet } from '../model/selectionSetTracker.js';
import { calculateResultLength, selectionSetFromOffsets, selectionSetsEqual, validateSelectionOffsets } from '../commands/selectionSetEditOperations.js';
import { type IRange, Range } from '../core/range.js';
import { type ISelection, Selection, SelectionDirection } from '../core/selection.js';
import { normalizeTextLineEndings, TextModelChangeReason, type TextModelChange } from '../core/textChange.js';
import { TextEditHistoryMergeMode } from '../core/editOperation.js';
import type * as editorCommon from '../editorCommon.js';
import { TrackedRangeStickiness, type IIdentifiedSingleEditOperation, type ITextModel, type IValidEditOperation } from '../model.js';
import { TextModel } from "../model/textModel.js";
import { EditorCommandHistoryMode, type EditorEditCommand, type TextSelectionOffsets } from "../commands/editorEditCommand.js";
import { CursorChangeReason } from '../cursorEvents.js';
import { EditSources, type TextModelEditSource } from '../textModelEditSource.js';
import { UndoRedoGroup } from '../../../platform/undoRedo/common/undoRedo.js';

export interface CursorSelectionSetChange {
	readonly selections: SelectionSet;
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

const DEFAULT_CURSOR_HISTORY_LIMIT = 100;
const DEFAULT_SELECTION_HISTORY_LIMIT = 1_000;

interface CompositionHost {
	isActive(): boolean;
	assertActive(): void;
	apply(command: EditorEditCommand): TextModelChange | undefined;
	commit(): void;
	cancel(): TextModelChange | undefined;
}

interface SelectionHistoryEntry {
	readonly before: SelectionSet;
	readonly after: SelectionSet;
}

interface ActiveComposition {
	readonly historyGroup: UndoRedoGroup;
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
	private readonly selectionHistoryLimit: number;
	private readonly cursorHistoryLimit: number;
	private readonly readOnlyMode: boolean;
	private readonly changeEmitter =
		this._register(new Emitter<CursorSelectionSetChange>());
	private readonly cursors: SelectionSetTracker;
	private readonly selectionHistory =
		new Map<number, SelectionHistoryEntry>();
	private readonly selectionHistoryOrder: number[] = [];
	private readonly cursorHistory: SelectionSet[] = [];
	private currentSelections: SelectionSet;
	private activeHistoryGroup: UndoRedoGroup | undefined;
	private activeHistoryMode: EditorCommandHistoryMode | undefined;
	private activeComposition: ActiveComposition | undefined;
	private executingCommand = false;

	readonly onDidChange: Event<CursorSelectionSetChange> =
		this.changeEmitter.event;

	constructor(
		private readonly model: TextModel,
		initialSelections: SelectionSet,
		options: CursorsControllerOptions = {},
	) {
		super();
		this.selectionHistoryLimit = readLimit(options.selectionHistoryLimit, DEFAULT_SELECTION_HISTORY_LIMIT, 'selectionHistoryLimit');
		this.cursorHistoryLimit = readLimit(options.cursorHistoryLimit, DEFAULT_CURSOR_HISTORY_LIMIT, 'cursorHistoryLimit');
		if (options.readOnly !== undefined && typeof options.readOnly !== 'boolean') throw new TypeError('Editor read-only mode must be boolean');
		this.readOnlyMode = options.readOnly ?? false;
		this.currentSelections = initialSelections;
		this.cursors = this._register(new SelectionSetTracker(model, initialSelections));
		try {
			this.installSelections(initialSelections);
			this._register(model.onDidChangeContent(change => this.acceptModelChange(change)));
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

	get selections(): SelectionSet {
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

	setSelections(selections: SelectionSet): void {
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
	setCursorSelections(selections: SelectionSet): void {
		this.assertNotDisposed();
		this.assertNoActiveComposition("set cursor selections");
		this.breakHistoryGroup();
		if (selectionSetsEqual(this.currentSelections, selections)) return;
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
		if (this.readOnlyMode) return undefined;
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
		const initialRange = initialSelections.primary;
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

		const after = selectionSetFromOffsets(
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
		selections: SelectionSet,
		reason?: CursorChangeReason,
	): void {
		validateSelectionSet(this.model, selections);
		const previous = this.currentSelections;
		this.cursors.setSelections(selections);
		this.currentSelections = selections;
		if (reason !== undefined && !selectionSetsEqual(previous, selections)) {
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

	private rememberCursorSelections(selections: SelectionSet): void {
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

}

interface IExecContext {
	readonly model: ITextModel;
	readonly selectionsBefore: Selection[];
	readonly trackedRanges: string[];
	readonly trackedRangesDirection: SelectionDirection[];
}

interface ICommandData {
	operations: IIdentifiedSingleEditOperation[];
	hadTrackedEditOperation: boolean;
}

interface ICommandsData {
	operations: IIdentifiedSingleEditOperation[];
	hadTrackedEditOperation: boolean;
}

export class CommandExecutor {
	public static executeCommands(model: ITextModel, selectionsBefore: Selection[], commands: (editorCommon.ICommand | null)[], editReason: TextModelEditSource = EditSources.unknown({ name: 'executeCommands' })): Selection[] | null {
		const context: IExecContext = {
			model,
			selectionsBefore,
			trackedRanges: [],
			trackedRangesDirection: [],
		};
		try {
			return this._innerExecuteCommands(context, commands, editReason);
		} finally {
			for (const trackedRange of context.trackedRanges) {
				context.model._setTrackedRange(trackedRange, null, TrackedRangeStickiness.AlwaysGrowsWhenTypingAtEdges);
			}
		}
	}

	private static _innerExecuteCommands(context: IExecContext, commands: (editorCommon.ICommand | null)[], editReason: TextModelEditSource): Selection[] | null {
		if (this._arrayIsEmpty(commands)) return null;

		const commandsData = this._getEditOperations(context, commands);
		if (commandsData.operations.length === 0) return null;

		const loserCursorsMap = this._getLoserCursorMap(commandsData.operations);
		if (Object.prototype.hasOwnProperty.call(loserCursorsMap, '0')) {
			console.warn('Ignoring commands');
			return null;
		}

		const filteredOperations = commandsData.operations.filter(operation => !Object.prototype.hasOwnProperty.call(loserCursorsMap, operation.identifier!.major.toString()));
		if (commandsData.hadTrackedEditOperation && filteredOperations.length > 0) filteredOperations[0]._isTracked = true;

		let selectionsAfter = context.model.pushEditOperations(context.selectionsBefore, filteredOperations, inverseEditOperations => {
			const groupedInverseEditOperations: IValidEditOperation[][] = context.selectionsBefore.map(() => []);
			for (const operation of inverseEditOperations) {
				if (operation.identifier) groupedInverseEditOperations[operation.identifier.major].push(operation);
			}
			return context.selectionsBefore.map((selection, index) => {
				const inverseOperations = groupedInverseEditOperations[index];
				if (inverseOperations.length === 0) return selection;
				inverseOperations.sort((left, right) => left.identifier!.minor - right.identifier!.minor);
				return commands[index]!.computeCursorState(context.model, {
					getInverseEditOperations: () => inverseOperations,
					getTrackedSelection: id => {
						const trackedRangeIndex = Number.parseInt(id, 10);
						const range = context.model._getTrackedRange(context.trackedRanges[trackedRangeIndex])!;
						return context.trackedRangesDirection[trackedRangeIndex] === SelectionDirection.LTR
							? new Selection(range.startLineNumber, range.startColumn, range.endLineNumber, range.endColumn)
							: new Selection(range.endLineNumber, range.endColumn, range.startLineNumber, range.startColumn);
					},
				});
			});
		}, undefined, editReason) ?? context.selectionsBefore;

		const losingCursors = Object.keys(loserCursorsMap).map(Number).sort((left, right) => right - left);
		for (const losingCursor of losingCursors) selectionsAfter.splice(losingCursor, 1);
		return selectionsAfter;
	}

	private static _arrayIsEmpty(commands: (editorCommon.ICommand | null)[]): boolean {
		return commands.every(command => command === null);
	}

	private static _getEditOperations(context: IExecContext, commands: (editorCommon.ICommand | null)[]): ICommandsData {
		let operations: IIdentifiedSingleEditOperation[] = [];
		let hadTrackedEditOperation = false;
		for (let index = 0; index < commands.length; index += 1) {
			const command = commands[index];
			if (!command) continue;
			const commandData = this._getEditOperationsFromCommand(context, index, command);
			operations = operations.concat(commandData.operations);
			hadTrackedEditOperation ||= commandData.hadTrackedEditOperation;
		}
		return { operations, hadTrackedEditOperation };
	}

	private static _getEditOperationsFromCommand(context: IExecContext, majorIdentifier: number, command: editorCommon.ICommand): ICommandData {
		const operations: IIdentifiedSingleEditOperation[] = [];
		let operationMinor = 0;
		const addEditOperation = (range: IRange, text: string | null, forceMoveMarkers = false): void => {
			if (Range.isEmpty(range) && text === '') return;
			operations.push({
				identifier: { major: majorIdentifier, minor: operationMinor++ },
				range,
				text,
				forceMoveMarkers,
				isAutoWhitespaceEdit: command.insertsAutoWhitespace,
			});
		};

		let hadTrackedEditOperation = false;
		const addTrackedEditOperation = (range: IRange, text: string | null, forceMoveMarkers?: boolean): void => {
			hadTrackedEditOperation = true;
			addEditOperation(range, text, forceMoveMarkers);
		};

		const trackSelection = (rawSelection: ISelection, trackPreviousOnEmpty?: boolean): string => {
			const selection = Selection.liftSelection(rawSelection);
			let stickiness: TrackedRangeStickiness;
			if (!selection.isEmpty()) {
				stickiness = TrackedRangeStickiness.NeverGrowsWhenTypingAtEdges;
			} else if (typeof trackPreviousOnEmpty === 'boolean') {
				stickiness = trackPreviousOnEmpty ? TrackedRangeStickiness.GrowsOnlyWhenTypingBefore : TrackedRangeStickiness.GrowsOnlyWhenTypingAfter;
			} else {
				stickiness = selection.startColumn === context.model.getLineMaxColumn(selection.startLineNumber)
					? TrackedRangeStickiness.GrowsOnlyWhenTypingBefore
					: TrackedRangeStickiness.GrowsOnlyWhenTypingAfter;
			}
			const index = context.trackedRanges.length;
			context.trackedRanges[index] = context.model._setTrackedRange(null, selection, stickiness);
			context.trackedRangesDirection[index] = selection.getDirection();
			return index.toString();
		};

		try {
			command.getEditOperations(context.model, { addEditOperation, addTrackedEditOperation, trackSelection });
		} catch (error) {
			onUnexpectedError(error);
			return { operations: [], hadTrackedEditOperation: false };
		}
		return { operations, hadTrackedEditOperation };
	}

	private static _getLoserCursorMap(rawOperations: IIdentifiedSingleEditOperation[]): Record<string, boolean> {
		const operations = rawOperations.slice().sort((left, right) => -Range.compareRangesUsingEnds(left.range, right.range));
		const loserCursorsMap: Record<string, boolean> = {};
		for (let index = 1; index < operations.length; index += 1) {
			const previousOperation = operations[index - 1];
			const currentOperation = operations[index];
			if (!Range.getStartPosition(previousOperation.range).isBefore(Range.getEndPosition(currentOperation.range))) continue;

			const losingMajor = Math.max(previousOperation.identifier!.major, currentOperation.identifier!.major);
			loserCursorsMap[losingMajor.toString()] = true;
			for (let operationIndex = 0; operationIndex < operations.length; operationIndex += 1) {
				if (operations[operationIndex].identifier!.major !== losingMajor) continue;
				operations.splice(operationIndex, 1);
				if (operationIndex < index) index -= 1;
				operationIndex -= 1;
			}
			if (index > 0) index -= 1;
		}
		return loserCursorsMap;
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

function readLimit(value: number | undefined, defaultValue: number, name: string): number {
	const limit = value ?? defaultValue;
	if (!Number.isSafeInteger(limit) || limit < 0) throw new RangeError(`${name} must be a non-negative safe integer`);
	return limit;
}
