import { addDisposableListener } from "../../../../../base/browser/dom.js";
import { getWindow } from "../../../../../base/browser/window.js";
import { DisposableOwner, ResettableDisposableGroup } from "../../../../../base/common/lifecycle.js";
import { type EditorSelectionController } from "../../../common/cursor/editorSelectionController.js";
import { createEditorColumnSelectionSet } from "../../../common/cursor/columnSelection.js";
import { SelectionDirection, TextSelection, TextSelectionSet } from "../../../common/core/selection.js";
import { TextPosition, TextRange } from "../../../common/core/text.js";
import { type TextModel } from "../../../common/model/textModel.js";
import { TrackedRangeStickiness, type TrackedRange } from "../../../common/model/trackedRange.js";
import { getWordSelectionRange } from "../../../common/cursor/wordBoundary.js";
import { type EditorViewport } from "../../../browser/view/editorViewport.js";
import { PointerAutoScroller } from "./pointerAutoScroll.js";
import { EditorHitTargetKind, type EditorHitTarget } from "../../../browser/view/pointerHitTest.js";
import { PointerMultiCursorModifier, combineAlphaPointerSelection, findAlphaPointerToggleCandidate, isAlphaPointerMultiCursorGesture, readAlphaPointerMultiCursorModifier } from "../common/pointerMultiCursor.js";

enum PointerSelectionKind {
  Character = "character",
  Word = "word",
  WholeLine = "wholeLine",
  ExtendToWord = "extendToWord",
  ExtendToLine = "extendToLine",
  Column = "column",
}

interface ActivePointerSelection {
  readonly kind: PointerSelectionKind;
  readonly pointerId: number | undefined;
  readonly anchor: TrackedRange;
  readonly columnFallbackAnchor: TrackedRange | undefined;
  readonly additionalSelections: AdditionalPointerSelections | undefined;
  columnMoved: boolean;
}

interface TrackedPointerSelection {
  readonly range: TrackedRange;
  readonly direction: SelectionDirection;
}

interface AdditionalPointerSelections {
  readonly selections: readonly TrackedPointerSelection[];
  readonly primaryIndex: number;
  readonly toggleCandidateIndex: number | undefined;
}

export interface PointerSelectionControllerOptions {
  readonly multiCursorModifier?: PointerMultiCursorModifier;
  /** Resolves the current language-specific word pattern for double-click selection. */
  readonly wordPattern?: () => RegExp | undefined;
}

/**
 * Browser pointer policy for one Alpha viewport and selection controller.
 *
 * The adapter owns pointer listeners and capture only. Text and selections
 * remain owned by the supplied common-layer controller.
 */
export class PointerSelectionController extends DisposableOwner {
  private readonly dragListeners =
    this.own(new ResettableDisposableGroup());
  private readonly multiCursorModifier: PointerMultiCursorModifier;
  private readonly wordPattern: (() => RegExp | undefined) | undefined;
  private activeSelection: ActivePointerSelection | undefined;
  private autoScroller: PointerAutoScroller | undefined;

  constructor(
    private readonly viewport: EditorViewport,
    private readonly selectionController: EditorSelectionController,
    options: PointerSelectionControllerOptions = {},
  ) {
    super();
    try {
      this.multiCursorModifier = readAlphaPointerMultiCursorModifier(
        options.multiCursorModifier,
      );
      if (options.wordPattern !== undefined && typeof options.wordPattern !== "function") {
        throw new TypeError("Alpha pointer word pattern resolver must be a function");
      }
      this.wordPattern = options.wordPattern;
    } catch (error) {
      this.dispose();
      throw error;
    }
    if (viewport.textModel !== selectionController.textModel) {
      this.dispose();
      throw new TypeError(
        "Alpha pointer and selection controllers must share one text model",
      );
    }
    this.own(addDisposableListener(
      viewport.element,
      "pointerdown",
      event => this.beginPointerSelection(event),
    ));
    this.own(addDisposableListener(viewport.element, "contextmenu", event => this.handleContextMenu(event)));
    this.defer(() => this.stopPointerSelection());
  }

  private beginPointerSelection(event: PointerEvent): void {
    if (event.defaultPrevented || event.button !== 0) return;
    const hitTarget = this.viewport.getTargetAtClientPoint(event);
    if (!hitTarget) return;
    event.preventDefault();
    this.viewport.element.focus({ preventScroll: true });
    this.stopPointerSelection();
    const pointerId = readPointerId(event);
    const addSelection = isAlphaPointerMultiCursorGesture(
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
      if (this.activeSelection.kind !== PointerSelectionKind.Column) {
        this.applyHitTarget(hitTarget);
      }
      this.capturePointer(pointerId);

      const targetWindow = getWindow(this.viewport.element);
      this.autoScroller = this.dragListeners.add(
        new PointerAutoScroller(
          targetWindow,
          this.viewport,
          target => this.applyHitTarget(target),
        ),
      );
      this.dragListeners.add(addDisposableListener(
        targetWindow,
        "pointermove",
        event => this.updatePointerSelection(event),
      ));
      this.dragListeners.add(addDisposableListener(
        targetWindow,
        "pointerup",
        event => this.finishPointerSelection(event),
      ));
      this.dragListeners.add(addDisposableListener(
        targetWindow,
        "pointercancel",
        event => this.cancelPointerSelection(event),
      ));
      this.dragListeners.add(addDisposableListener(
        targetWindow,
        "blur",
        () => this.stopPointerSelection(),
        { once: true },
      ));
    } catch (error) {
      this.stopPointerSelection();
      throw error;
    }
  }

  private handleContextMenu(event: MouseEvent): void {
    const hitTarget = this.viewport.getTargetAtClientPoint(event);
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
  ): ActivePointerSelection {
    let kind: PointerSelectionKind;
    let anchorRange: TextRange;
    if (column && clickCount === 1) {
      kind = PointerSelectionKind.Column;
      anchorRange = TextRange.emptyAt(hitTarget.position);
    } else if (hitTarget.kind === EditorHitTargetKind.Gutter) {
      if (extend) {
        kind = PointerSelectionKind.ExtendToLine;
        anchorRange = TextRange.emptyAt(
          this.selectionController.selections.primary.anchor,
        );
      } else {
        kind = PointerSelectionKind.WholeLine;
        anchorRange = TextRange.emptyAt(
          lineStart(hitTarget.position.lineIndex),
        );
      }
    } else if (clickCount >= 3) {
      if (extend) {
        kind = PointerSelectionKind.ExtendToLine;
        anchorRange = TextRange.emptyAt(
          this.selectionController.selections.primary.anchor,
        );
      } else {
        kind = PointerSelectionKind.WholeLine;
        anchorRange = TextRange.emptyAt(
          lineStart(hitTarget.position.lineIndex),
        );
      }
    } else if (clickCount === 2) {
      if (extend) {
        kind = PointerSelectionKind.ExtendToWord;
        anchorRange = TextRange.emptyAt(
          this.selectionController.selections.primary.anchor,
        );
      } else {
        kind = PointerSelectionKind.Word;
        anchorRange = getWordSelectionRange(this.viewport.textModel, hitTarget.position, this.wordPattern?.());
      }
    } else {
      kind = PointerSelectionKind.Character;
      anchorRange = TextRange.emptyAt(extend
        ? this.selectionController.selections.primary.anchor
        : hitTarget.position);
    }
    const anchor = this.dragListeners.add(this.viewport.textModel.trackRange(
      anchorRange,
      TrackedRangeStickiness.NeverGrowsAtEdges,
    ));
    const initialSelection = pointerSelectionForTarget(
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
      columnFallbackAnchor: kind === PointerSelectionKind.Column
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

  private trackAdditionalSelections(initialSelection: TextSelection): AdditionalPointerSelections {
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
      toggleCandidateIndex: findAlphaPointerToggleCandidate(
        base,
        initialSelection,
      ),
    };
  }

  private updatePointerSelection(event: PointerEvent): void {
    if (!this.accepts(event)) return;
    const hitTarget = this.viewport.getNearestTargetAtClientPoint(event);
    if (hitTarget) {
      if (this.activeSelection?.kind === PointerSelectionKind.Column) {
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
      if (active?.kind === PointerSelectionKind.Column && !active.columnMoved) {
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
    if (active.kind === PointerSelectionKind.Column) {
      this.selectionController.setSelections(createEditorColumnSelectionSet(
        this.viewport.textModel,
        anchorRange.start,
        hitTarget.position,
      ));
      return;
    }
    const selection = pointerSelectionForTarget(
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
    this.selectionController.setSelections(combineAlphaPointerSelection(
      base,
      selection,
      additional.toggleCandidateIndex,
    ));
  }

  private applyColumnFallback(active: ActivePointerSelection, hitTarget: EditorHitTarget): void {
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

  private capturePointer(pointerId: number | undefined): void {
    if (
      pointerId !== undefined &&
      typeof this.viewport.element.setPointerCapture === "function"
    ) {
      this.viewport.element.setPointerCapture(pointerId);
    }
  }

  private stopPointerSelection(): void {
    const active = this.activeSelection;
    this.activeSelection = undefined;
    this.autoScroller = undefined;
    this.dragListeners.clear();
    const pointerId = active?.pointerId;
    if (
      pointerId !== undefined &&
      typeof this.viewport.element.hasPointerCapture === "function" &&
      this.viewport.element.hasPointerCapture(pointerId)
    ) {
      this.viewport.element.releasePointerCapture(pointerId);
    }
  }
}

/** Returns whether a context-menu point belongs to existing selected content. */
export function isPositionInSelections(position: TextPosition, selections: TextSelectionSet): boolean {
  return selections.selections.some(selection => !selection.collapsed && position.compareTo(selection.range.start) >= 0 && position.compareTo(selection.range.end) < 0);
}

function pointerSelectionForTarget(kind: PointerSelectionKind, model: TextModel, anchorRange: TextRange, hitTarget: EditorHitTarget, wordPattern: RegExp | undefined): TextSelection {
  const anchor = anchorRange.start;
  if (kind === PointerSelectionKind.Character) {
    return TextSelection.from(anchor, hitTarget.position);
  }
  if (kind === PointerSelectionKind.Column) return TextSelection.collapsedAt(anchor);
  if (kind === PointerSelectionKind.Word) {
    return wordSelection(model, anchorRange, hitTarget.position, wordPattern);
  }
  if (kind === PointerSelectionKind.WholeLine) {
    return wholeLineSelection(
      model,
      anchor.lineIndex,
      hitTarget.position.lineIndex,
    );
  }
  if (kind === PointerSelectionKind.ExtendToWord) {
    return extendSelectionToWord(model, anchor, hitTarget.position, wordPattern);
  }
  return extendSelectionToLine(model, anchor, hitTarget.position.lineIndex);
}

function trackedSelectionSet(additional: AdditionalPointerSelections): TextSelectionSet {
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
