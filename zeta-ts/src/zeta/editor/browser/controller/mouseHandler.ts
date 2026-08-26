import { DisposableOwner, ResettableDisposableGroup } from "../../../base/common/lifecycle.js";
import { type EditorSelectionController } from "../../common/cursor/editorSelectionController.js";
import { createEditorColumnSelectionSet } from "../../common/cursor/columnSelection.js";
import { SelectionDirection, TextSelection, TextSelectionSet } from "../../common/core/selection.js";
import { TextPosition, TextRange } from "../../common/core/text.js";
import { type TextModel } from "../../common/model/textModel.js";
import { TrackedRangeStickiness, type TrackedRange } from "../../common/model/trackedRange.js";
import { getWordSelectionRange } from "../../common/cursor/wordBoundary.js";
import { type EditorViewport } from "../view.js";
import { DragScrolling } from "./dragScrolling.js";
import { PointerHandler } from "./pointerHandler.js";
import { MouseTargetFactory, MouseTargetKind } from "./mouseTarget.js";
import { EditorHitTargetKind, type EditorHitTarget } from "../../common/viewModel/pointerHitTest.js";
import { PointerMultiCursorModifier, combineStanzaPointerSelection, findStanzaPointerToggleCandidate, isStanzaPointerMultiCursorGesture, readStanzaPointerMultiCursorModifier } from "../../common/cursor/pointerMultiCursor.js";

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
export class MouseHandler extends DisposableOwner {
	private readonly dragListeners =
		this.own(new ResettableDisposableGroup());
	private readonly pointerHandler: PointerHandler;
	private readonly mouseTargetFactory: MouseTargetFactory;
	private readonly multiCursorModifier: PointerMultiCursorModifier;
	private readonly wordPattern: (() => RegExp | undefined) | undefined;
	private activeSelection: ActiveMouseSelection | undefined;
	private autoScroller: DragScrolling | undefined;

	constructor(
		private readonly viewport: EditorViewport,
		private readonly selectionController: EditorSelectionController,
		options: MouseHandlerOptions = {},
	) {
		super();
		try {
			this.multiCursorModifier = readStanzaPointerMultiCursorModifier(
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
		this.pointerHandler = this.own(new PointerHandler(viewport.element));
		this.mouseTargetFactory = new MouseTargetFactory(viewport);
		this.own(this.pointerHandler.onDidPointerDown(event => this.beginPointerSelection(event)));
		this.own(this.pointerHandler.onDidContextMenu(event => this.handleContextMenu(event)));
		this.defer(() => this.stopPointerSelection());
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
		const addSelection = isStanzaPointerMultiCursorGesture(
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
		this.selectionController.setSelections(TextSelectionSet.single(TextSelection.collapsedAt(hitTarget.position)));
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
		let anchorRange: TextRange;
		if (column && clickCount === 1) {
			kind = MouseSelectionKind.Column;
			anchorRange = TextRange.emptyAt(hitTarget.position);
		} else if (hitTarget.kind === EditorHitTargetKind.Gutter) {
			if (extend) {
				kind = MouseSelectionKind.ExtendToLine;
				anchorRange = TextRange.emptyAt(
					this.selectionController.selections.primary.anchor,
				);
			} else {
				kind = MouseSelectionKind.WholeLine;
				anchorRange = TextRange.emptyAt(
					lineStart(hitTarget.position.lineIndex),
				);
			}
		} else if (clickCount >= 3) {
			if (extend) {
				kind = MouseSelectionKind.ExtendToLine;
				anchorRange = TextRange.emptyAt(
					this.selectionController.selections.primary.anchor,
				);
			} else {
				kind = MouseSelectionKind.WholeLine;
				anchorRange = TextRange.emptyAt(
					lineStart(hitTarget.position.lineIndex),
				);
			}
		} else if (clickCount === 2) {
			if (extend) {
				kind = MouseSelectionKind.ExtendToWord;
				anchorRange = TextRange.emptyAt(
					this.selectionController.selections.primary.anchor,
				);
			} else {
				kind = MouseSelectionKind.Word;
				anchorRange = getWordSelectionRange(this.viewport.textModel, hitTarget.position, this.wordPattern?.());
			}
		} else {
			kind = MouseSelectionKind.Character;
			anchorRange = TextRange.emptyAt(extend
				? this.selectionController.selections.primary.anchor
				: hitTarget.position);
		}
		const anchor = this.dragListeners.add(this.viewport.textModel.trackRange(
			anchorRange,
			TrackedRangeStickiness.NeverGrowsAtEdges,
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
					TextRange.emptyAt(this.selectionController.selections.primary.anchor),
					TrackedRangeStickiness.NeverGrowsAtEdges,
				))
				: undefined,
			additionalSelections: addSelection
				? this.trackAdditionalSelections(initialSelection)
				: undefined,
			columnMoved: false,
		};
	}

	private trackAdditionalSelections(initialSelection: TextSelection): AdditionalMouseSelections {
		const base = this.selectionController.selections;
		return {
			selections: base.selections.map(selection => ({
				range: this.dragListeners.add(this.viewport.textModel.trackRange(
					selection.range,
					TrackedRangeStickiness.NeverGrowsAtEdges,
				)),
				direction: selection.direction,
			})),
			primaryIndex: base.primaryIndex,
			toggleCandidateIndex: findStanzaPointerToggleCandidate(
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
			this.selectionController.setSelections(createEditorColumnSelectionSet(
				this.viewport.textModel,
				anchorRange.start,
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
			this.selectionController.setSelections(TextSelectionSet.single(selection));
			return;
		}
		const base = trackedSelectionSet(additional);
		this.selectionController.setSelections(combineStanzaPointerSelection(
			base,
			selection,
			additional.toggleCandidateIndex,
		));
	}

	private applyColumnFallback(active: ActiveMouseSelection, hitTarget: EditorHitTarget): void {
		const anchor = active.columnFallbackAnchor?.range.start;
		if (!anchor) return;
		this.selectionController.setSelections(TextSelectionSet.single(
			TextSelection.from(anchor, hitTarget.position),
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
export function isPositionInSelections(position: TextPosition, selections: TextSelectionSet): boolean {
	return selections.selections.some(selection => !selection.collapsed && position.compareTo(selection.range.start) >= 0 && position.compareTo(selection.range.end) < 0);
}

function selectionForTarget(kind: MouseSelectionKind, model: TextModel, anchorRange: TextRange, hitTarget: EditorHitTarget, wordPattern: RegExp | undefined): TextSelection {
	const anchor = anchorRange.start;
	if (kind === MouseSelectionKind.Character) {
		return TextSelection.from(anchor, hitTarget.position);
	}
	if (kind === MouseSelectionKind.Column) return TextSelection.collapsedAt(anchor);
	if (kind === MouseSelectionKind.Word) {
		return wordSelection(model, anchorRange, hitTarget.position, wordPattern);
	}
	if (kind === MouseSelectionKind.WholeLine) {
		return wholeLineSelection(
			model,
			anchor.lineIndex,
			hitTarget.position.lineIndex,
		);
	}
	if (kind === MouseSelectionKind.ExtendToWord) {
		return extendSelectionToWord(model, anchor, hitTarget.position, wordPattern);
	}
	return extendSelectionToLine(model, anchor, hitTarget.position.lineIndex);
}

function trackedSelectionSet(additional: AdditionalMouseSelections): TextSelectionSet {
	return TextSelectionSet.withPrimary(
		additional.selections.map(selection => {
			const range = selection.range.range;
			return selection.direction === SelectionDirection.Backward
				? TextSelection.from(range.end, range.start)
				: TextSelection.from(range.start, range.end);
		}),
		additional.primaryIndex,
	);
}

function wordSelection(model: TextModel, anchorRange: TextRange, activePosition: TextPosition, wordPattern: RegExp | undefined): TextSelection {
	const activeRange = getWordSelectionRange(model, activePosition, wordPattern);
	return activeRange.start.compareTo(anchorRange.start) < 0
		? TextSelection.from(anchorRange.end, activeRange.start)
		: TextSelection.from(anchorRange.start, activeRange.end);
}

function extendSelectionToWord(model: TextModel, anchor: TextPosition, activePosition: TextPosition, wordPattern: RegExp | undefined): TextSelection {
	const activeRange = getWordSelectionRange(model, activePosition, wordPattern);
	const active = activeRange.start.compareTo(anchor) < 0
		? activeRange.start
		: activeRange.end;
	return TextSelection.from(anchor, active);
}

function wholeLineSelection(
	model: TextModel,
	anchorLineIndex: number,
	activeLineIndex: number,
): TextSelection {
	if (activeLineIndex >= anchorLineIndex) {
		return TextSelection.from(
			lineStart(anchorLineIndex),
			lineEndExclusive(model, activeLineIndex),
		);
	}
	return TextSelection.from(
		lineEndExclusive(model, anchorLineIndex),
		lineStart(activeLineIndex),
	);
}

function extendSelectionToLine(
	model: TextModel,
	anchor: TextPosition,
	activeLineIndex: number,
): TextSelection {
	const active = activeLineIndex < anchor.lineIndex
		? lineStart(activeLineIndex)
		: lineEndExclusive(model, activeLineIndex);
	return TextSelection.from(anchor, active);
}

function lineStart(lineIndex: number): TextPosition {
	return TextPosition.at(lineIndex, 0);
}

function lineEndExclusive(
	model: TextModel,
	lineIndex: number,
): TextPosition {
	if (lineIndex + 1 < model.lineCount) {
		return TextPosition.at(lineIndex + 1, 0);
	}
	return TextPosition.at(
		lineIndex,
		model.getLineContent(lineIndex).length,
	);
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
