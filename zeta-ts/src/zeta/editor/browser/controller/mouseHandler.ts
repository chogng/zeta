import { Disposable, DisposableStore, toDisposable } from "../../../base/common/lifecycle.js";
import { type CursorsController } from "../../common/cursor/cursor.js";
import { ColumnSelection } from "../../common/cursor/cursorColumnSelection.js";
import { WordOperations } from '../../common/cursor/cursorWordOperations.js';
import { SelectionStartKind, SingleCursorState } from '../../common/cursorCommon.js';
import { SelectionDirection, Selection } from "../../common/core/selection.js";
import { SelectionSet } from "../../common/cursor/selectionSet.js";
import { Position } from "../../common/core/position.js";
import { Range } from "../../common/core/range.js";
import { type TextModel } from "../../common/model/textModel.js";
import { type TrackedRange } from "../../common/model/trackedRange.js";
import { type View } from "../view.js";
import { BidirectionalDragScrolling } from "./bidirectionalDragScrolling.js";
import { PointerEventRouter } from "./pointerEventRouter.js";
import { SemanticMouseTargetFactory, SemanticMouseTargetKind } from "./semanticMouseTarget.js";
import { EditorHitTargetKind, type EditorHitTarget } from "../../common/viewModel/pointerHitTest.js";
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

export enum PointerMultiCursorModifier {
	Alt = "alt",
	ControlOrMeta = "controlOrMeta",
}

interface PointerModifierState {
	readonly altKey: boolean;
	readonly ctrlKey: boolean;
	readonly metaKey: boolean;
	readonly shiftKey: boolean;
}

export interface MouseHandlerOptions {
	readonly multiCursorModifier?: PointerMultiCursorModifier;
}

/**
 * Browser mouse and pointer policy for one Stanza viewport and selection controller.
 *
 * PointerEventRouter owns browser dispatch/capture. This controller owns gesture
 * policy and maps semantic hit targets to common selection state.
 */
export class EditorPointerSelectionHandler extends Disposable {
	private readonly dragListeners =
		this._register(new DisposableStore());
	private readonly pointerHandler: PointerEventRouter;
	private readonly mouseTargetFactory: SemanticMouseTargetFactory;
	private readonly multiCursorModifier: PointerMultiCursorModifier;
	private activeSelection: ActiveMouseSelection | undefined;
	private autoScroller: BidirectionalDragScrolling | undefined;

	constructor(
		private readonly viewport: View,
		private readonly selectionController: CursorsController,
		options: MouseHandlerOptions = {},
	) {
		super();
		try {
			this.multiCursorModifier = readPointerMultiCursorModifier(
				options.multiCursorModifier,
			);
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
		this.pointerHandler = this._register(new PointerEventRouter(viewport.element));
		this.mouseTargetFactory = new SemanticMouseTargetFactory(viewport);
		this._register(this.pointerHandler.onDidPointerDown(event => this.beginPointerSelection(event)));
		this._register(this.pointerHandler.onDidContextMenu(event => this.handleContextMenu(event)));
		this._register(toDisposable(() => this.stopPointerSelection()));
	}

	private beginPointerSelection(event: PointerEvent): void {
		if (event.defaultPrevented || event.button !== 0) return;
		const target = this.mouseTargetFactory.create(event);
		if (!target || target.kind === SemanticMouseTargetKind.Scrollbar || target.kind === SemanticMouseTargetKind.Widget || target.kind === SemanticMouseTargetKind.ViewZone) return;
		const hitTarget = target.editorTarget;
		if (!hitTarget) return;
		event.preventDefault();
		this.viewport.element.focus({ preventScroll: true });
		this.stopPointerSelection();
		const pointerId = readPointerId(event);
		const addSelection = isPointerMultiCursorGesture(
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
				new BidirectionalDragScrolling(
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
		if (!target || target.kind === SemanticMouseTargetKind.Scrollbar || target.kind === SemanticMouseTargetKind.Widget || target.kind === SemanticMouseTargetKind.ViewZone) return;
		const hitTarget = target?.editorTarget;
		if (!hitTarget) return;
		this.viewport.element.focus({ preventScroll: true });
		if (isPositionInSelections(hitTarget.position, this.selectionController.selections)) return;
		this.selectionController.setSelections(SelectionSet.single(Selection.fromPositions(hitTarget.position)));
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
					this.selectionController.selections.primary.getSelectionStart(),
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
					this.selectionController.selections.primary.getSelectionStart(),
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
					this.selectionController.selections.primary.getSelectionStart(),
				);
			} else {
				kind = MouseSelectionKind.Word;
				anchorRange = mouseWordRange(this.viewport, hitTarget.position);
			}
		} else {
			kind = MouseSelectionKind.Character;
			anchorRange = Range.fromPositions(extend
				? this.selectionController.selections.primary.getSelectionStart()
				: hitTarget.position);
		}
		const anchor = this.dragListeners.add(this.viewport.textModel.trackRange(
			anchorRange,
			TrackedRangeStickiness.NeverGrowsWhenTypingAtEdges,
		));
		const initialSelection = selectionForTarget(
			kind,
			this.viewport,
			anchorRange,
			hitTarget,
		);
		return {
			kind,
			pointerId,
			anchor,
			columnFallbackAnchor: kind === MouseSelectionKind.Column
				? this.dragListeners.add(this.viewport.textModel.trackRange(
					Range.fromPositions(this.selectionController.selections.primary.getSelectionStart()),
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
			selections: base.selections.map(selection => ({
				range: this.dragListeners.add(this.viewport.textModel.trackRange(
					selection,
					TrackedRangeStickiness.NeverGrowsWhenTypingAtEdges,
				)),
				direction: selection.getDirection(),
			})),
			primaryIndex: base.primaryIndex,
			toggleCandidateIndex: findPointerToggleCandidate(
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
			const coordinates = this.viewport.coordinatesConverter;
			const model = this.viewport.cursorModel;
			const config = this.viewport.cursorConfig;
			const anchor = coordinates.convertModelPositionToViewPosition(anchorRange.getStartPosition());
			const target = coordinates.convertModelPositionToViewPosition(hitTarget.position);
			const result = ColumnSelection.columnSelect(
				config,
				model,
				anchor.lineNumber,
				config.visibleColumnFromColumn(model, anchor),
				target.lineNumber,
				config.visibleColumnFromColumn(model, target),
			);
			this.selectionController.setSelections(SelectionSet.withPrimary(result.viewStates.map(state => Selection.fromPositions(
				coordinates.convertViewPositionToModelPosition(state.selection.getSelectionStart()),
				coordinates.convertViewPositionToModelPosition(state.position),
			)), 0));
			return;
		}
		const selection = selectionForTarget(
			active.kind,
			this.viewport,
			anchorRange,
			hitTarget,
		);
		const additional = active.additionalSelections;
		if (!additional) {
			this.selectionController.setSelections(SelectionSet.single(selection));
			return;
		}
		const base = trackedSelectionSet(additional);
		this.selectionController.setSelections(combinePointerSelection(
			base,
			selection,
			additional.toggleCandidateIndex,
		));
	}

	private applyColumnFallback(active: ActiveMouseSelection, hitTarget: EditorHitTarget): void {
		const anchor = active.columnFallbackAnchor?.range.getStartPosition();
		if (!anchor) return;
		this.selectionController.setSelections(SelectionSet.single(
			Selection.fromPositions(anchor, hitTarget.position),
		));
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
export function isPositionInSelections(position: Position, selections: SelectionSet): boolean {
	return selections.selections.some(selection => !selection.isEmpty() && Position.compare(position, selection.getStartPosition()) >= 0 && Position.compare(position, selection.getEndPosition()) < 0);
}

function readPointerMultiCursorModifier(value: PointerMultiCursorModifier | undefined): PointerMultiCursorModifier {
	const resolved = value ?? PointerMultiCursorModifier.Alt;
	if (resolved !== PointerMultiCursorModifier.Alt && resolved !== PointerMultiCursorModifier.ControlOrMeta) {
		throw new TypeError('Unknown Stanza pointer multi-cursor modifier');
	}
	return resolved;
}

function isPointerMultiCursorGesture(state: PointerModifierState, modifier: PointerMultiCursorModifier): boolean {
	if (state.shiftKey) return false;
	if (modifier === PointerMultiCursorModifier.Alt) return state.altKey && !state.ctrlKey && !state.metaKey;
	return (state.ctrlKey || state.metaKey) && !state.altKey;
}

function findPointerToggleCandidate(base: SelectionSet, selection: Selection): number | undefined {
	const index = base.selections.findIndex(candidate => selectionsHaveSameRange(candidate, selection));
	return index >= 0 ? index : undefined;
}

function combinePointerSelection(base: SelectionSet, active: Selection, toggleCandidateIndex: number | undefined): SelectionSet {
	if (toggleCandidateIndex !== undefined && selectionsHaveSameRange(base.selections[toggleCandidateIndex]!, active)) {
		if (base.selections.length === 1) return base;
		const selections = base.selections.filter((_, index) => index !== toggleCandidateIndex);
		return SelectionSet.withPrimary(selections, primaryAfterRemoval(base.primaryIndex, toggleCandidateIndex, selections.length));
	}

	const retained = toggleCandidateIndex === undefined
		? [...base.selections]
		: base.selections.filter((_, index) => index !== toggleCandidateIndex);
	const duplicateIndex = retained.findIndex(selection => selectionsHaveSameRange(selection, active));
	if (duplicateIndex >= 0) return SelectionSet.withPrimary(retained, duplicateIndex);
	const nonOverlapping = retained.filter(selection => !selectionRangesOverlap(selection, active));
	if (nonOverlapping.length === 0) return SelectionSet.single(active);
	return SelectionSet.withPrimary([...nonOverlapping, active], nonOverlapping.length);
}

function selectionsHaveSameRange(left: Selection, right: Selection): boolean {
	return Position.compare(left.getStartPosition(), right.getStartPosition()) === 0 && Position.compare(left.getEndPosition(), right.getEndPosition()) === 0;
}

function selectionRangesOverlap(left: Selection, right: Selection): boolean {
	if (left.isEmpty()) return pointOverlapsRange(left.getStartPosition(), right);
	if (right.isEmpty()) return pointOverlapsRange(right.getStartPosition(), left);
	return Position.compare(left.getStartPosition(), right.getEndPosition()) < 0 && Position.compare(right.getStartPosition(), left.getEndPosition()) < 0;
}

function pointOverlapsRange(point: Position, selection: Selection): boolean {
	if (selection.isEmpty()) return Position.compare(point, selection.getStartPosition()) === 0;
	return Position.compare(point, selection.getStartPosition()) >= 0 && Position.compare(point, selection.getEndPosition()) < 0;
}

function primaryAfterRemoval(primaryIndex: number, removedIndex: number, remainingCount: number): number {
	if (primaryIndex < removedIndex) return primaryIndex;
	if (primaryIndex > removedIndex) return primaryIndex - 1;
	return Math.min(removedIndex, remainingCount - 1);
}

function selectionForTarget(kind: MouseSelectionKind, viewport: View, anchorRange: Range, hitTarget: EditorHitTarget): Selection {
	const model = viewport.textModel;
	const anchor = anchorRange.getStartPosition();
	if (kind === MouseSelectionKind.Character) {
		return Selection.fromPositions(anchor, hitTarget.position);
	}
	if (kind === MouseSelectionKind.Column) return Selection.fromPositions(anchor);
	if (kind === MouseSelectionKind.Word) {
		return wordSelection(viewport, anchorRange, hitTarget.position);
	}
	if (kind === MouseSelectionKind.WholeLine) {
		return wholeLineSelection(
			model,
			anchor.lineNumber,
			hitTarget.position.lineNumber,
		);
	}
	if (kind === MouseSelectionKind.ExtendToWord) {
		return extendSelectionToWord(viewport, anchor, hitTarget.position);
	}
	return extendSelectionToLine(model, anchor, hitTarget.position.lineNumber);
}

function trackedSelectionSet(additional: AdditionalMouseSelections): SelectionSet {
	return SelectionSet.withPrimary(
		additional.selections.map(selection => {
			const range = selection.range.range;
			return selection.direction === SelectionDirection.RTL
				? Selection.fromPositions(range.getEndPosition(), range.getStartPosition())
				: Selection.fromPositions(range.getStartPosition(), range.getEndPosition());
		}),
		additional.primaryIndex,
	);
}

function wordSelection(viewport: View, anchorRange: Range, activePosition: Position): Selection {
	const activeRange = mouseWordRange(viewport, activePosition);
	return Position.compare(activeRange.getStartPosition(), anchorRange.getStartPosition()) < 0
		? Selection.fromPositions(anchorRange.getEndPosition(), activeRange.getStartPosition())
		: Selection.fromPositions(anchorRange.getStartPosition(), activeRange.getEndPosition());
}

function extendSelectionToWord(viewport: View, anchor: Position, activePosition: Position): Selection {
	const activeRange = mouseWordRange(viewport, activePosition);
	const active = Position.compare(activeRange.getStartPosition(), anchor) < 0
		? activeRange.getStartPosition()
		: activeRange.getEndPosition();
	return Selection.fromPositions(anchor, active);
}

function mouseWordRange(viewport: View, position: Position): Range {
	const cursor = new SingleCursorState(Range.fromPositions(position), SelectionStartKind.Simple, 0, position, 0);
	return WordOperations.word(viewport.cursorConfig, viewport.cursorModel, cursor, false, position).selectionStart;
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
