import { addDisposableListener } from "../../../base/browser/dom.js";
import { getWindow } from "../../../base/browser/window.js";
import { DisposableOwner, ResettableDisposableGroup } from "../../../base/common/lifecycle.js";
import { type EditorSelectionController } from "../common/editorSelectionController.js";
import { SelectionDirection, TextSelection, TextSelectionSet } from "../common/selection.js";
import { TextPosition, TextRange } from "../common/text.js";
import { type TextModel } from "../common/textModel.js";
import { TrackedRangeStickiness, type TrackedRange } from "../common/trackedRange.js";
import { getWordSelectionRange } from "../common/wordBoundary.js";
import { type AlphaEditorViewport } from "./alphaEditorViewport.js";
import { AlphaPointerAutoScroller } from "./pointerAutoScroll.js";
import { AlphaEditorHitTargetKind, type AlphaEditorHitTarget } from "./pointerHitTest.js";
import { AlphaPointerMultiCursorModifier, combineAlphaPointerSelection, findAlphaPointerToggleCandidate, isAlphaPointerMultiCursorGesture, readAlphaPointerMultiCursorModifier } from "./pointerMultiCursor.js";

enum PointerSelectionKind {
  Character = "character",
  Word = "word",
  WholeLine = "wholeLine",
  ExtendToWord = "extendToWord",
  ExtendToLine = "extendToLine",
}

interface ActivePointerSelection {
  readonly kind: PointerSelectionKind;
  readonly pointerId: number | undefined;
  readonly anchor: TrackedRange;
  readonly additionalSelections: AdditionalPointerSelections | undefined;
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

export interface AlphaPointerSelectionControllerOptions {
  readonly multiCursorModifier?: AlphaPointerMultiCursorModifier;
}

/**
 * Browser pointer policy for one Alpha viewport and selection controller.
 *
 * The adapter owns pointer listeners and capture only. Text and selections
 * remain owned by the supplied common-layer controller.
 */
export class AlphaPointerSelectionController extends DisposableOwner {
  private readonly dragListeners =
    this.own(new ResettableDisposableGroup());
  private readonly multiCursorModifier: AlphaPointerMultiCursorModifier;
  private activeSelection: ActivePointerSelection | undefined;
  private autoScroller: AlphaPointerAutoScroller | undefined;

  constructor(
    private readonly viewport: AlphaEditorViewport,
    private readonly selectionController: EditorSelectionController,
    options: AlphaPointerSelectionControllerOptions = {},
  ) {
    super();
    try {
      this.multiCursorModifier = readAlphaPointerMultiCursorModifier(
        options.multiCursorModifier,
      );
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
    this.defer(() => this.stopPointerSelection());
  }

  private beginPointerSelection(event: PointerEvent): void {
    if (event.button !== 0) return;
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
        readClickCount(event),
        addSelection,
      );
      this.applyHitTarget(hitTarget);
      this.capturePointer(pointerId);

      const targetWindow = getWindow(this.viewport.element);
      this.autoScroller = this.dragListeners.add(
        new AlphaPointerAutoScroller(
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

  private createActiveSelection(
    hitTarget: AlphaEditorHitTarget,
    pointerId: number | undefined,
    extend: boolean,
    clickCount: number,
    addSelection: boolean,
  ): ActivePointerSelection {
    let kind: PointerSelectionKind;
    let anchorRange: TextRange;
    if (hitTarget.kind === AlphaEditorHitTargetKind.Gutter) {
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
        anchorRange = getWordSelectionRange(
          this.viewport.textModel,
          hitTarget.position,
        );
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
    );
    return {
      kind,
      pointerId,
      anchor,
      additionalSelections: addSelection
        ? this.trackAdditionalSelections(initialSelection)
        : undefined,
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
    if (hitTarget) this.applyHitTarget(hitTarget);
    this.autoScroller?.updatePointer(event);
  }

  private finishPointerSelection(event: PointerEvent): void {
    if (!this.accepts(event)) return;
    const hitTarget = this.viewport.getNearestTargetAtClientPoint(event);
    if (hitTarget) this.applyHitTarget(hitTarget);
    this.stopPointerSelection();
  }

  private cancelPointerSelection(event: PointerEvent): void {
    if (!this.accepts(event)) return;
    this.stopPointerSelection();
  }

  private applyHitTarget(hitTarget: AlphaEditorHitTarget): void {
    const active = this.activeSelection;
    if (!active) return;
    const anchorRange = active.anchor.range;
    const selection = pointerSelectionForTarget(
      active.kind,
      this.viewport.textModel,
      anchorRange,
      hitTarget,
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

function pointerSelectionForTarget(kind: PointerSelectionKind, model: TextModel, anchorRange: TextRange, hitTarget: AlphaEditorHitTarget): TextSelection {
  const anchor = anchorRange.start;
  if (kind === PointerSelectionKind.Character) {
    return TextSelection.from(anchor, hitTarget.position);
  }
  if (kind === PointerSelectionKind.Word) {
    return wordSelection(model, anchorRange, hitTarget.position);
  }
  if (kind === PointerSelectionKind.WholeLine) {
    return wholeLineSelection(
      model,
      anchor.lineIndex,
      hitTarget.position.lineIndex,
    );
  }
  if (kind === PointerSelectionKind.ExtendToWord) {
    return extendSelectionToWord(model, anchor, hitTarget.position);
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

function wordSelection(model: TextModel, anchorRange: TextRange, activePosition: TextPosition): TextSelection {
  const activeRange = getWordSelectionRange(model, activePosition);
  return activeRange.start.compareTo(anchorRange.start) < 0
    ? TextSelection.from(anchorRange.end, activeRange.start)
    : TextSelection.from(anchorRange.start, activeRange.end);
}

function extendSelectionToWord(model: TextModel, anchor: TextPosition, activePosition: TextPosition): TextSelection {
  const activeRange = getWordSelectionRange(model, activePosition);
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
