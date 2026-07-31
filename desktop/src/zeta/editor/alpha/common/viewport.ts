import { Emitter, type Event } from "../../../base/common/event.js";
import { type ISize } from "../../../base/common/layout.js";
import { DisposableOwner } from "../../../base/common/lifecycle.js";
import { type TextModelChange } from "./text.js";
import { TextModel } from "./textModel.js";

export interface EditorScrollPosition {
  readonly left: number;
  readonly top: number;
}

export interface EditorLineRange {
  readonly startLineIndex: number;
  readonly endLineIndexExclusive: number;
}

export interface EditorViewportLayout {
  readonly modelVersion: number;
  readonly lineHeight: number;
  readonly viewportSize: ISize;
  readonly contentSize: ISize;
  readonly scrollPosition: EditorScrollPosition;
  readonly maximumScrollPosition: EditorScrollPosition;
  readonly visibleLines: EditorLineRange;
  readonly renderLines: EditorLineRange;
  readonly renderTop: number;
}

export enum EditorViewportChangeReason {
  Model = "model",
  ViewportSize = "viewportSize",
  ContentWidth = "contentWidth",
  LineHeight = "lineHeight",
  Scroll = "scroll",
}

export interface EditorViewportChange {
  readonly reason: EditorViewportChangeReason;
  readonly layout: EditorViewportLayout;
  readonly modelChange?: TextModelChange;
}

export interface EditorViewportOptions {
  readonly lineHeight: number;
  readonly overscanLineCount?: number;
}

/**
 * DOM-free fixed-line-height viewport state for one text model.
 *
 * Browser code owns measurement and supplies viewport/content widths. This
 * model owns scroll clamping and visible/render line ranges.
 */
export class EditorViewportModel extends DisposableOwner {
  private readonly changeEmitter =
    this.own(new Emitter<EditorViewportChange>());
  private readonly overscanLineCount: number;
  private viewportSize: ISize = Object.freeze({ width: 0, height: 0 });
  private measuredContentWidth = 0;
  private requestedScrollPosition: EditorScrollPosition =
    Object.freeze({ left: 0, top: 0 });
  private currentLineHeight: number;
  private currentLayout: EditorViewportLayout;

  readonly onDidChange: Event<EditorViewportChange> =
    this.changeEmitter.event;

  constructor(
    private readonly model: TextModel,
    options: EditorViewportOptions,
  ) {
    const lineHeight = positiveFinite(options.lineHeight, "lineHeight");
    const overscanLineCount = nonNegativeSafeInteger(
      options.overscanLineCount ?? 2,
      "overscanLineCount",
    );
    model.version;
    super();
    this.currentLineHeight = lineHeight;
    this.overscanLineCount = overscanLineCount;
    this.currentLayout = this.createLayout();
    this.own(model.onDidChange(change => {
      this.publish(EditorViewportChangeReason.Model, change);
    }));
  }

  get layout(): EditorViewportLayout {
    return this.currentLayout;
  }

  setViewportSize(size: ISize): EditorViewportLayout {
    const next = readSize(size, "viewportSize");
    if (sizesEqual(this.viewportSize, next)) return this.currentLayout;
    this.viewportSize = next;
    this.publish(EditorViewportChangeReason.ViewportSize);
    return this.currentLayout;
  }

  setContentWidth(width: number): EditorViewportLayout {
    const next = nonNegativeFinite(width, "contentWidth");
    if (this.measuredContentWidth === next) return this.currentLayout;
    this.measuredContentWidth = next;
    this.publish(EditorViewportChangeReason.ContentWidth);
    return this.currentLayout;
  }

  setLineHeight(lineHeight: number): EditorViewportLayout {
    const next = positiveFinite(lineHeight, "lineHeight");
    if (this.currentLineHeight === next) return this.currentLayout;
    const topLine = this.currentLayout.scrollPosition.top /
      this.currentLineHeight;
    this.currentLineHeight = next;
    this.requestedScrollPosition = Object.freeze({
      left: this.currentLayout.scrollPosition.left,
      top: topLine * next,
    });
    this.publish(EditorViewportChangeReason.LineHeight);
    return this.currentLayout;
  }

  setScrollPosition(
    position: EditorScrollPosition,
  ): EditorViewportLayout {
    const next = readScrollPosition(position);
    if (scrollPositionsEqual(
      this.requestedScrollPosition,
      next,
    )) {
      return this.currentLayout;
    }
    this.requestedScrollPosition = next;
    this.publish(EditorViewportChangeReason.Scroll);
    return this.currentLayout;
  }

  private publish(
    reason: EditorViewportChangeReason,
    modelChange?: TextModelChange,
  ): void {
    const next = this.createLayout();
    this.requestedScrollPosition = next.scrollPosition;
    if (layoutsEqual(this.currentLayout, next)) return;
    this.currentLayout = next;
    this.changeEmitter.fire(Object.freeze({
      reason,
      layout: next,
      modelChange,
    }));
  }

  private createLayout(): EditorViewportLayout {
    const contentSize = Object.freeze({
      width: Math.max(
        this.viewportSize.width,
        this.measuredContentWidth,
      ),
      height: Math.max(
        this.viewportSize.height,
        this.model.lineCount * this.currentLineHeight,
      ),
    });
    const maximumScrollPosition = Object.freeze({
      left: contentSize.width - this.viewportSize.width,
      top: contentSize.height - this.viewportSize.height,
    });
    const scrollPosition = Object.freeze({
      left: clamp(
        this.requestedScrollPosition.left,
        0,
        maximumScrollPosition.left,
      ),
      top: clamp(
        this.requestedScrollPosition.top,
        0,
        maximumScrollPosition.top,
      ),
    });
    const visibleLines = visibleLineRange(
      this.model.lineCount,
      this.currentLineHeight,
      this.viewportSize.height,
      scrollPosition.top,
    );
    const hasVisibleLines =
      visibleLines.startLineIndex <
      visibleLines.endLineIndexExclusive;
    const renderLines = Object.freeze(hasVisibleLines
      ? {
        startLineIndex: Math.max(
          0,
          visibleLines.startLineIndex - this.overscanLineCount,
        ),
        endLineIndexExclusive: Math.min(
          this.model.lineCount,
          visibleLines.endLineIndexExclusive +
            this.overscanLineCount,
        ),
      }
      : visibleLines);
    return Object.freeze({
      modelVersion: this.model.version,
      lineHeight: this.currentLineHeight,
      viewportSize: this.viewportSize,
      contentSize,
      scrollPosition,
      maximumScrollPosition,
      visibleLines,
      renderLines,
      renderTop:
        renderLines.startLineIndex * this.currentLineHeight,
    });
  }
}

function visibleLineRange(
  lineCount: number,
  lineHeight: number,
  viewportHeight: number,
  scrollTop: number,
): EditorLineRange {
  if (viewportHeight === 0) {
    return Object.freeze({
      startLineIndex: 0,
      endLineIndexExclusive: 0,
    });
  }
  return Object.freeze({
    startLineIndex: Math.min(
      lineCount - 1,
      Math.floor(scrollTop / lineHeight),
    ),
    endLineIndexExclusive: Math.min(
      lineCount,
      Math.ceil((scrollTop + viewportHeight) / lineHeight),
    ),
  });
}

function readSize(size: ISize, name: string): ISize {
  if (!size || typeof size !== "object") {
    throw new TypeError(`${name} must be a size`);
  }
  return Object.freeze({
    width: nonNegativeFinite(size.width, `${name}.width`),
    height: nonNegativeFinite(size.height, `${name}.height`),
  });
}

function readScrollPosition(
  position: EditorScrollPosition,
): EditorScrollPosition {
  if (!position || typeof position !== "object") {
    throw new TypeError("scrollPosition must be an object");
  }
  return Object.freeze({
    left: finite(position.left, "scrollPosition.left"),
    top: finite(position.top, "scrollPosition.top"),
  });
}

function positiveFinite(value: number, name: string): number {
  const result = finite(value, name);
  if (result <= 0) throw new RangeError(`${name} must be positive`);
  return result;
}

function nonNegativeFinite(value: number, name: string): number {
  const result = finite(value, name);
  if (result < 0) throw new RangeError(`${name} must be non-negative`);
  return result;
}

function finite(value: number, name: string): number {
  if (!Number.isFinite(value)) {
    throw new RangeError(`${name} must be finite`);
  }
  return value;
}

function nonNegativeSafeInteger(value: number, name: string): number {
  if (!Number.isSafeInteger(value) || value < 0) {
    throw new RangeError(`${name} must be a non-negative safe integer`);
  }
  return value;
}

function clamp(value: number, minimum: number, maximum: number): number {
  return Math.min(Math.max(value, minimum), maximum);
}

function sizesEqual(left: ISize, right: ISize): boolean {
  return left.width === right.width &&
    left.height === right.height;
}

function scrollPositionsEqual(
  left: EditorScrollPosition,
  right: EditorScrollPosition,
): boolean {
  return left.left === right.left && left.top === right.top;
}

function layoutsEqual(
  left: EditorViewportLayout,
  right: EditorViewportLayout,
): boolean {
  return left.modelVersion === right.modelVersion &&
    left.lineHeight === right.lineHeight &&
    sizesEqual(left.viewportSize, right.viewportSize) &&
    sizesEqual(left.contentSize, right.contentSize) &&
    scrollPositionsEqual(
      left.scrollPosition,
      right.scrollPosition,
    ) &&
    scrollPositionsEqual(
      left.maximumScrollPosition,
      right.maximumScrollPosition,
    ) &&
    lineRangesEqual(left.visibleLines, right.visibleLines) &&
    lineRangesEqual(left.renderLines, right.renderLines);
}

function lineRangesEqual(
  left: EditorLineRange,
  right: EditorLineRange,
): boolean {
  return left.startLineIndex === right.startLineIndex &&
    left.endLineIndexExclusive === right.endLineIndexExclusive;
}
