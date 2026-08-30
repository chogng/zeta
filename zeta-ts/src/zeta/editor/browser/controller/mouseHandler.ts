import { Disposable, DisposableStore, toDisposable } from "../../../base/common/lifecycle.js";
import { type CursorsController } from "../../common/cursor/cursor.js";
import { ColumnSelection } from "../../common/cursor/cursorColumnSelection.js";
import { SelectionDirection, Selection } from "../../common/core/selection.js";
import { Position } from "../../common/core/position.js";
import { Range } from "../../common/core/range.js";
import { type TextModel } from "../../common/model/textModel.js";
import { type TrackedRange } from "../../common/model/trackedRange.js";
import { WordOperations } from "../../common/cursor/cursorWordOperations.js";
import { type View } from "../view.js";
import { DragScrolling } from "./dragScrolling.js";
import { PointerHandler } from "./pointerHandler.js";
import { MouseTargetFactory, MouseTargetKind } from "./mouseTarget.js";
import { EditorHitTargetKind, type EditorHitTarget } from "../../common/viewModel/pointerHitTest.js";
import { CursorMoveCommands, PointerMultiCursorModifier, type PointerModifierState } from "../../common/cursor/cursorMoveCommands.js";
import { TrackedRangeStickiness } from '../../common/model.js';

enum MouseSelectionKind {
	Character = "character",
	Word = "word",
	WholeLine = "wholeLine",
	ExtendToWord = "extendToWord",
	ExtendToLine = "extendToLine",
	Column = "column",
}

interface ActiveMouseSelection {
	readonly kind: MouseSelectionKind;
	readonly pointerId: number | undefined;
	readonly anchor: TrackedRange;
	readonly columnFallbackAnchor: TrackedRange | undefined;
	readonly additionalSelections: AdditionalMouseSelections | undefined;
	columnMoved: boolean;
}

interface TrackedMouseSelection {
	readonly range: TrackedRange;
	readonly direction: SelectionDirection;
}

interface AdditionalMouseSelections {
	readonly selections: readonly TrackedMouseSelection[];
	readonly primaryIndex: number;
	readonly toggleCandidateIndex: number | undefined;
}

export interface MouseHandlerOptions {
	readonly multiCursorModifier?: PointerMultiCursorModifier;
	/** Resolves the current language-specific word pattern for double-click selection. */
	readonly wordPattern?: () => RegExp | undefined;
}

/**
 * Browser mouse and pointer policy for one Stanza viewport and selection controller.
 *
 * PointerHandler owns browser dispatch/capture. This controller owns gesture
 * policy and maps semantic hit targets to common selection state.
 */
export class MouseHandler extends Disposable {
	private readonly dragListeners =
		this._register(new DisposableStore());
	private readonly pointerHandler: PointerHandler;
	private readonly mouseTargetFactory: MouseTargetFactory;
	private readonly multiCursorModifier: PointerMultiCursorModifier;
	private readonly wordPattern: (() => RegExp | undefined) | undefined;
	private activeSelection: ActiveMouseSelection | undefined;
	private autoScroller: DragScrolling | undefined;

	constructor(
		private readonly viewport: View,
		private readonly selectionController: CursorsController,
		options: MouseHandlerOptions = {},
	) {
		super();
		try {
			this.multiCursorModifier = CursorMoveCommands.readPointerMultiCursorModifier(
				options.multiCursorModifier,
			);
			if (options.wordPattern !== undefined && typeof options.wordPattern !== "function") {
				throw new TypeError("Stanza pointer word pattern resolver must be a function");
			}
			this.wordPattern = options.wordPattern;
		} catch (error) {
			this.dispose();
			throw error;
		}
		if (viewport.textModel !== selectionController.textModel) {
			this.dispose();
			throw new TypeError(
				"Stanza pointer and selection controllers must share one text model",
			);
		}
		this.pointerHandler = this._register(new PointerHandler(viewport.element));
		this.mouseTargetFactory = new MouseTargetFactory(viewport);
		this._register(this.pointerHandler.onDidPointerDown(event => this.beginPointerSelection(event)));
		this._register(this.pointerHandler.onDidContextMenu(event => this.handleContextMenu(event)));
		this._register(toDisposable(() => this.stopPointerSelection()));
	}

	private beginPointerSelection(event: PointerEvent): void {
		if (event.defaultPrevented || event.button !== 0) return;
		const target = this.mouseTargetFactory.create(event);
		if (!target || target.kind === MouseTargetKind.Scrollbar || target.kind === MouseTargetKind.Widget || target.kind === MouseTargetKind.ViewZone) return;
		const hitTarget = target.editorTarget;
		if (!hitTarget) return;
		event.preventDefault();
		this.viewport.element.focus({ preventScroll: true });
		this.stopPointerSelection();
		const pointerId = readPointerId(event);
		const addSelection = CursorMoveCommands.isPointerMultiCursorGesture(
			event,
			this.multiCursorModifier,
		);
		try {
			this.activeSelection = this.createActiveSelection(
				hitTarget,
				pointerId,
				event.shiftKey,
				event.altKey && event.shiftKey && hitTarget.kind !== EditorHitTargetKind.Gutter,
				readClickCount(event),
				addSelection,
			);
			if (this.activeSelection.kind !== MouseSelectionKind.Column) {
				this.applyHitTarget(hitTarget);
			}
			this.pointerHandler.capturePointer(pointerId);

			const targetWindow = this.pointerHandler.targetWindow;
			this.autoScroller = this.dragListeners.add(
				new DragScrolling(
					targetWindow,
					this.viewport,
					target => this.applyHitTarget(target),
				),
			);
			this.dragListeners.add(this.pointerHandler.startTracking(pointerId, {
				onMove: event => this.updatePointerSelection(event),
				onUp: event => this.finishPointerSelection(event),
				onCancel: event => this.cancelPointerSelection(event),
				onBlur: () => this.stopPointerSelection(),
			}));
		} catch (error) {
			this.stopPointerSelection();
			throw error;
		}
	}

	private handleContextMenu(event: MouseEvent): void {
		const target = this.mouseTargetFactory.create(event, true);
		if (!target || target.kind === MouseTargetKind.Scrollbar || target.kind === MouseTargetKind.Widget || target.kind === MouseTargetKind.ViewZone) return;
		const hitTarget = target?.editorTarget;
		if (!hitTarget) return;
		this.viewport.element.focus({ preventScroll: true });
		if (isPositionInSelections(hitTarget.position, this.selectionController.selections)) return;
		this.selectionController.setSelections([Selection.fromPositions(hitTarget.position)]);
	}

	private createActiveSelection(
		hitTarget: EditorHitTarget,
		pointerId: number | undefined,
		extend: boolean,
		column: boolean,
		clickCount: number,
		addSelection: boolean,
	): ActiveMouseSelection {
		let kind: MouseSelectionKind;
		let anchorRange: Range;
		if (column && clickCount === 1) {
			kind = MouseSelectionKind.Column;
			anchorRange = Range.fromPositions(hitTarget.position);
		} else if (hitTarget.kind === EditorHitTargetKind.Gutter) {
			if (extend) {
				kind = MouseSelectionKind.ExtendToLine;
				anchorRange = Range.fromPositions(
					this.selectionController.selections[0]!.getSelectionStart(),
				);
			} else {
				kind = MouseSelectionKind.WholeLine;
				anchorRange = Range.fromPositions(
					lineStart(hitTarget.position.lineNumber),
				);
			}
		} else if (clickCount >= 3) {
			if (extend) {
				kind = MouseSelectionKind.ExtendToLine;
				anchorRange = Range.fromPositions(
					this.selectionController.selections[0]!.getSelectionStart(),
				);
			} else {
				kind = MouseSelectionKind.WholeLine;
				anchorRange = Range.fromPositions(
					lineStart(hitTarget.position.lineNumber),
				);
			}
		} else if (clickCount === 2) {
			if (extend) {
				kind = MouseSelectionKind.ExtendToWord;
				anchorRange = Range.fromPositions(
					this.selectionController.selections[0]!.getSelectionStart(),
				);
			} else {
				kind = MouseSelectionKind.Word;
				anchorRange = WordOperations.getWordSelectionRange(this.viewport.textModel, hitTarget.position, this.wordPattern?.());
			}
		} else {
			kind = MouseSelectionKind.Character;
			anchorRange = Range.fromPositions(extend
				? this.selectionController.selections[0]!.getSelectionStart()
				: hitTarget.position);
		}
		const anchor = this.dragListeners.add(this.viewport.textModel.trackRange(
			anchorRange,
			TrackedRangeStickiness.NeverGrowsWhenTypingAtEdges,
		));
		const initialSelection = selectionForTarget(
			kind,
			this.viewport.textModel,
			anchorRange,
			hitTarget,
			this.wordPattern?.(),
		);
		return {
			kind,
			pointerId,
			anchor,
			columnFallbackAnchor: kind === MouseSelectionKind.Column
				? this.dragListeners.add(this.viewport.textModel.trackRange(
					Range.fromPositions(this.selectionController.selections[0]!.getSelectionStart()),
					TrackedRangeStickiness.NeverGrowsWhenTypingAtEdges,
				))
				: undefined,
			additionalSelections: addSelection
				? this.trackAdditionalSelections(initialSelection)
				: undefined,
			columnMoved: false,
		};
	}

	private trackAdditionalSelections(initialSelection: Selection): AdditionalMouseSelections {
		const base = this.selectionController.selections;
		return {
			selections: base.map(selection => ({
				range: this.dragListeners.add(this.viewport.textModel.trackRange(
					selection,
					TrackedRangeStickiness.NeverGrowsWhenTypingAtEdges,
				)),
				direction: selection.getDirection(),
			})),
			primaryIndex: 0,
			toggleCandidateIndex: CursorMoveCommands.findPointerToggleCandidate(
				base,
				initialSelection,
			),
		};
	}

	private updatePointerSelection(event: PointerEvent): void {
		if (!this.accepts(event)) return;
		const hitTarget = this.viewport.getNearestTargetAtClientPoint(event);
		if (hitTarget) {
			if (this.activeSelection?.kind === MouseSelectionKind.Column) {
				this.activeSelection.columnMoved = true;
			}
			this.applyHitTarget(hitTarget);
		}
		this.autoScroller?.updatePointer(event);
	}

	private finishPointerSelection(event: PointerEvent): void {
		if (!this.accepts(event)) return;
		const hitTarget = this.viewport.getNearestTargetAtClientPoint(event);
		if (hitTarget) {
			const active = this.activeSelection;
			if (active?.kind === MouseSelectionKind.Column && !active.columnMoved) {
				this.applyColumnFallback(active, hitTarget);
			} else {
				this.applyHitTarget(hitTarget);
			}
		}
		this.stopPointerSelection();
	}

	private cancelPointerSelection(event: PointerEvent): void {
		if (!this.accepts(event)) return;
		this.stopPointerSelection();
	}

	private applyHitTarget(hitTarget: EditorHitTarget): void {
		const active = this.activeSelection;
		if (!active) return;
		const anchorRange = active.anchor.range;
		if (active.kind === MouseSelectionKind.Column) {
			this.selectionController.setSelections(ColumnSelection.columnSelect(
				this.viewport.textModel,
				anchorRange.getStartPosition(),
				hitTarget.position,
			));
			return;
		}
		const selection = selectionForTarget(
			active.kind,
			this.viewport.textModel,
			anchorRange,
			hitTarget,
			this.wordPattern?.(),
		);
		const additional = active.additionalSelections;
		if (!additional) {
			this.selectionController.setSelections([selection]);
			return;
		}
		const base = trackedSelectionSet(additional);
		this.selectionController.setSelections(CursorMoveCommands.combinePointerSelection(
			base,
			selection,
			additional.toggleCandidateIndex,
		));
	}

	private applyColumnFallback(active: ActiveMouseSelection, hitTarget: EditorHitTarget): void {
		const anchor = active.columnFallbackAnchor?.range.getStartPosition();
		if (!anchor) return;
		this.selectionController.setSelections([Selection.fromPositions(anchor, hitTarget.position)]);
	}

	private accepts(event: PointerEvent): boolean {
		const active = this.activeSelection;
		if (!active) return false;
		const pointerId = readPointerId(event);
		return active.pointerId === undefined ||
			pointerId === undefined ||
			pointerId === active.pointerId;
	}

	private stopPointerSelection(): void {
		const active = this.activeSelection;
		this.activeSelection = undefined;
		this.autoScroller = undefined;
		this.dragListeners.clear();
		const pointerId = active?.pointerId;
		this.pointerHandler.releasePointer(pointerId);
	}
}

/** Returns whether a context-menu point belongs to existing selected content. */
export function isPositionInSelections(position: Position, selections: readonly Selection[]): boolean {
	return selections.some(selection => !selection.isEmpty() && Position.compare(position, selection.getStartPosition()) >= 0 && Position.compare(position, selection.getEndPosition()) < 0);
}

function selectionForTarget(kind: MouseSelectionKind, model: TextModel, anchorRange: Range, hitTarget: EditorHitTarget, wordPattern: RegExp | undefined): Selection {
	const anchor = anchorRange.getStartPosition();
	if (kind === MouseSelectionKind.Character) {
		return Selection.fromPositions(anchor, hitTarget.position);
	}
	if (kind === MouseSelectionKind.Column) return Selection.fromPositions(anchor);
	if (kind === MouseSelectionKind.Word) {
		return wordSelection(model, anchorRange, hitTarget.position, wordPattern);
	}
	if (kind === MouseSelectionKind.WholeLine) {
		return wholeLineSelection(
			model,
			anchor.lineNumber,
			hitTarget.position.lineNumber,
		);
	}
	if (kind === MouseSelectionKind.ExtendToWord) {
		return extendSelectionToWord(model, anchor, hitTarget.position, wordPattern);
	}
	return extendSelectionToLine(model, anchor, hitTarget.position.lineNumber);
}

function trackedSelectionSet(additional: AdditionalMouseSelections): readonly Selection[] {
	const selections = additional.selections.map(selection => {
			const range = selection.range.range;
			return selection.direction === SelectionDirection.RTL
				? Selection.fromPositions(range.getEndPosition(), range.getStartPosition())
				: Selection.fromPositions(range.getStartPosition(), range.getEndPosition());
		});
	return primaryFirst(selections, additional.primaryIndex);
}

function wordSelection(model: TextModel, anchorRange: Range, activePosition: Position, wordPattern: RegExp | undefined): Selection {
	const activeRange = WordOperations.getWordSelectionRange(model, activePosition, wordPattern);
	return Position.compare(activeRange.getStartPosition(), anchorRange.getStartPosition()) < 0
		? Selection.fromPositions(anchorRange.getEndPosition(), activeRange.getStartPosition())
		: Selection.fromPositions(anchorRange.getStartPosition(), activeRange.getEndPosition());
}

function primaryFirst(selections: readonly Selection[], primaryIndex: number): readonly Selection[] {
	if (primaryIndex === 0) return Object.freeze([...selections]);
	return Object.freeze([selections[primaryIndex]!, ...selections.slice(0, primaryIndex), ...selections.slice(primaryIndex + 1)]);
}

function extendSelectionToWord(model: TextModel, anchor: Position, activePosition: Position, wordPattern: RegExp | undefined): Selection {
	const activeRange = WordOperations.getWordSelectionRange(model, activePosition, wordPattern);
	const active = Position.compare(activeRange.getStartPosition(), anchor) < 0
		? activeRange.getStartPosition()
		: activeRange.getEndPosition();
	return Selection.fromPositions(anchor, active);
}

function wholeLineSelection(
	model: TextModel,
	anchorLineNumber: number,
	activeLineNumber: number,
): Selection {
	if (activeLineNumber >= anchorLineNumber) {
		return Selection.fromPositions(
			lineStart(anchorLineNumber),
			lineEndExclusive(model, activeLineNumber),
		);
	}
	return Selection.fromPositions(
		lineEndExclusive(model, anchorLineNumber),
		lineStart(activeLineNumber),
	);
}

function extendSelectionToLine(
	model: TextModel,
	anchor: Position,
	activeLineNumber: number,
): Selection {
	const active = activeLineNumber < anchor.lineNumber
		? lineStart(activeLineNumber)
		: lineEndExclusive(model, activeLineNumber);
	return Selection.fromPositions(anchor, active);
}

function lineStart(lineNumber: number): Position {
	return new Position(lineNumber, 1);
}

function lineEndExclusive(
	model: TextModel,
	lineNumber: number,
): Position {
	if (lineNumber < model.lineCount) {
		return new Position(lineNumber + 1, 1);
	}
	return new Position(lineNumber, model.getLineContent(lineNumber).length + 1);
}

function readPointerId(event: PointerEvent): number | undefined {
	return Number.isFinite(event.pointerId)
		? event.pointerId
		: undefined;
}

function readClickCount(event: PointerEvent): number {
	return Number.isSafeInteger(event.detail) && event.detail > 0
		? Math.min(event.detail, 3)
		: 1;
}
