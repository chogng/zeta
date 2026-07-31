import "./media/alphaEditorViewport.css";
import { addDisposableListener, reset } from "../../../base/browser/dom.js";
import { getClientArea } from "../../../base/browser/geometry.js";
import { type Event } from "../../../base/common/event.js";
import { type ISize } from "../../../base/common/layout.js";
import { DisposableOwner } from "../../../base/common/lifecycle.js";
import { type EditorSelectionController } from "../common/editorSelectionController.js";
import { type TextPosition, type TextRange } from "../common/text.js";
import { type TextModel } from "../common/textModel.js";
import { TrackedRangeStickiness, type TrackedRange } from "../common/trackedRange.js";
import { type EditorLineRange, type EditorScrollPosition, type EditorViewportChange, type EditorViewportLayout, EditorViewportModel } from "../common/viewport.js";
import { type AlphaDecorationSource, type AlphaResolvedDecoration } from "./decorationPresentation.js";
import { AlphaDomTextMeasurer, type AlphaTextMeasurer } from "./fontMetrics.js";
import { AlphaLineWidthIndex } from "./lineWidthIndex.js";
import { type AlphaClientPoint, type AlphaEditorHitTarget, hitTestAlphaEditorPoint } from "./pointerHitTest.js";
import { createAlphaRenderedLine, type AlphaRenderedLine } from "./renderedLine.js";
import { projectAlphaSemanticTokenLine, type AlphaResolvedSemanticToken, type AlphaSemanticTokenSource } from "./semanticTokenPresentation.js";
import { projectAlphaCompositionOverlay, projectAlphaDecorationOverlays, projectAlphaSelectionOverlays, type AlphaViewportOverlayContext } from "./viewportOverlayPresentation.js";

const GUTTER_HORIZONTAL_PADDING = 16;

export type AlphaEditorViewportPresentation = "document" | "embedded";

export interface AlphaEditorViewportOptions {
  readonly container: HTMLElement;
  readonly model: TextModel;
  readonly lineHeight: number;
  readonly overscanLineCount?: number;
  readonly ariaLabel?: string;
  readonly textMeasurer?: AlphaTextMeasurer;
  readonly selectionController?: EditorSelectionController;
  readonly decorationSources?: readonly AlphaDecorationSource[];
  readonly semanticTokenSource?: AlphaSemanticTokenSource;
  readonly presentation?: AlphaEditorViewportPresentation;
}

export interface AlphaEditorContentPosition {
  readonly left: number;
  readonly top: number;
  readonly height: number;
}

/**
 * Read-only browser projection of one Alpha text model.
 *
 * The common viewport owns layout math. This component owns the scroll host,
 * virtual line DOM, measurement inputs, and their lifecycle.
 */
export class AlphaEditorViewport extends DisposableOwner {
  readonly element: HTMLDivElement;
  readonly onDidChangeLayout: Event<EditorViewportChange>;
  private readonly model: TextModel;
  private readonly viewport: EditorViewportModel;
  private readonly contentElement: HTMLDivElement;
  private readonly linesElement: HTMLDivElement;
  private readonly textMetricsElement: HTMLSpanElement;
  private readonly textMeasurer: AlphaTextMeasurer;
  private readonly lineWidths: AlphaLineWidthIndex;
  private readonly selectionController: EditorSelectionController | undefined;
  private readonly decorationSources: readonly AlphaDecorationSource[];
  private readonly semanticTokenSource: AlphaSemanticTokenSource | undefined;
  private readonly presentation: AlphaEditorViewportPresentation;
  private readonly decorationSnapshots =
    new Map<AlphaDecorationSource, AlphaDecorationSource["decorations"]>();
  private renderedLines = new Map<number, AlphaRenderedLine>();
  private renderedRange: EditorLineRange = {
    startLineIndex: 0,
    endLineIndexExclusive: 0,
  };
  private renderedModelVersion = -1;
  private renderedLineHeight = -1;
  private compositionRange: TrackedRange | undefined;

  constructor(options: AlphaEditorViewportOptions) {
    const ownerDocument = options.container.ownerDocument;
    const viewport = new EditorViewportModel(options.model, {
      lineHeight: options.lineHeight,
      overscanLineCount: options.overscanLineCount,
    });
    super();
    this.model = options.model;
    this.viewport = this.own(viewport);
    this.onDidChangeLayout = viewport.onDidChange;
    this.element = ownerDocument.createElement("div");
    this.contentElement = ownerDocument.createElement("div");
    this.linesElement = ownerDocument.createElement("div");
    this.textMetricsElement = ownerDocument.createElement("span");
    this.selectionController = options.selectionController;
    this.semanticTokenSource = options.semanticTokenSource;
    this.presentation = options.presentation ?? "document";
    try {
      if (this.selectionController && this.selectionController.textModel !== this.model) {
        throw new TypeError(
          "Alpha viewport and selection controller must share one text model",
        );
      }
      if (this.semanticTokenSource && this.semanticTokenSource.textModel !== this.model) {
        throw new TypeError("Alpha viewport and semantic token source must share one text model");
      }
    } catch (error) {
      this.dispose();
      throw error;
    }
    this.decorationSources = Object.freeze([
      ...(options.decorationSources ?? []),
    ]);

    this.element.className = "zeta-alpha-editor";
    this.element.classList.add(`zeta-alpha-editor-${this.presentation}`);
    this.element.tabIndex = 0;
    this.element.setAttribute("role", "region");
    this.element.setAttribute("aria-label", options.ariaLabel ?? "Alpha editor");
    this.contentElement.className = "zeta-alpha-editor-content";
    this.linesElement.className = "zeta-alpha-editor-lines";
    this.textMetricsElement.className =
      "zeta-alpha-editor-text-metrics";
    this.textMetricsElement.setAttribute("aria-hidden", "true");
    this.contentElement.append(this.linesElement);
    this.element.append(this.contentElement, this.textMetricsElement);
    options.container.append(this.element);
    this.defer(() => this.element.remove());
    this.defer(() => this.compositionRange?.dispose());
    this.textMeasurer =
      options.textMeasurer ??
      new AlphaDomTextMeasurer(this.textMetricsElement);
    this.lineWidths = new AlphaLineWidthIndex(this.model, this.textMeasurer);
    viewport.setContentWidth(this.measuredContentWidth);

    this.own(viewport.onDidChange(({ layout }) => this.project(layout)));
    this.own(addDisposableListener(this.element, "scroll", () => {
      const layout = viewport.setScrollPosition({
        left: this.element.scrollLeft,
        top: this.element.scrollTop,
      });
      this.syncScrollPosition(layout);
    }));
    this.own(this.model.onDidChange(change => {
      this.lineWidths.applyModelChange(change);
      viewport.setContentWidth(this.measuredContentWidth);
    }));
    if (this.selectionController) {
      this.own(this.selectionController.onDidChange(() => {
        this.projectSelections(viewport.layout);
      }));
    }
    for (const source of this.decorationSources) {
      this.decorationSnapshots.set(source, source.decorations);
      this.own(source.onDidChange(() => {
        this.decorationSnapshots.set(source, source.decorations);
        this.projectDecorations(viewport.layout);
      }));
    }
    const semanticTokenSource = this.semanticTokenSource;
    if (semanticTokenSource) {
      this.own(semanticTokenSource.onDidChange(() => {
        this.projectVisibleLineText();
      }));
    }
    const fontSet = ownerDocument.fonts;
    if (fontSet) {
      this.own(addDisposableListener(fontSet, "loadingdone", () => {
        this.refreshFontMetrics();
      }));
    }

    const ResizeObserverConstructor = ownerDocument.defaultView?.ResizeObserver;
    if (ResizeObserverConstructor) {
      const observer = new ResizeObserverConstructor(([entry]) => {
        if (!entry) return;
        this.layout({
          width: entry.contentRect.width,
          height: entry.contentRect.height,
        });
      });
      observer.observe(this.element);
      this.defer(() => observer.disconnect());
    }

    this.project(viewport.layout);
    this.layout();
  }

  get viewportLayout(): EditorViewportLayout {
    return this.viewport.layout;
  }

  get textModel(): TextModel {
    return this.model;
  }

  layout(size: ISize = getClientArea(this.element)): EditorViewportLayout {
    this.refreshFontMetrics();
    const layout = this.viewport.setViewportSize(size);
    this.project(layout);
    return layout;
  }

  refreshFontMetrics(): EditorViewportLayout {
    if (!this.textMeasurer.refresh()) return this.viewport.layout;
    this.lineWidths.rebuild();
    const layout = this.viewport.setContentWidth(this.measuredContentWidth);
    this.project(layout);
    return layout;
  }

  setLineHeight(lineHeight: number): EditorViewportLayout {
    const layout = this.viewport.setLineHeight(lineHeight);
    this.project(layout);
    return layout;
  }

  scrollTo(position: EditorScrollPosition): EditorViewportLayout {
    const layout = this.viewport.setScrollPosition(position);
    this.project(layout);
    return layout;
  }

  revealPosition(position: TextPosition): EditorViewportLayout {
    this.model.offsetAt(position);
    const layout = this.viewport.layout;
    const lineTop = position.lineIndex * layout.lineHeight;
    const lineBottom = lineTop + layout.lineHeight;
    let top = layout.scrollPosition.top;
    if (lineTop < top) {
      top = lineTop;
    } else if (lineBottom > top + layout.viewportSize.height) {
      top = lineBottom - layout.viewportSize.height;
    }

    const line = this.model.getLineContent(position.lineIndex);
    const caretLeft = this.textLeft +
      this.textMeasurer.measureLineWidth(line.slice(0, position.columnIndex));
    const caretRight = caretLeft + Math.max(
      1,
      this.textMeasurer.measureLineWidth(" "),
    );
    let left = layout.scrollPosition.left;
    if (caretLeft < left + this.textLeft) {
      left = caretLeft - this.textLeft;
    } else if (caretRight > left + layout.viewportSize.width) {
      left = caretRight - layout.viewportSize.width;
    }
    return this.scrollTo({ left, top });
  }

  getPositionContentCoordinates(position: TextPosition): AlphaEditorContentPosition {
    this.model.offsetAt(position);
    return Object.freeze({
      left: this.textLeft + this.textMeasurer.measureLineWidth(
        this.model.getLineContent(position.lineIndex).slice(0, position.columnIndex),
      ),
      top: position.lineIndex * this.viewport.layout.lineHeight,
      height: this.viewport.layout.lineHeight,
    });
  }

  setCompositionRange(range: TextRange | undefined): void {
    const next = range
      ? this.model.trackRange(
        range,
        TrackedRangeStickiness.NeverGrowsAtEdges,
      )
      : undefined;
    this.compositionRange?.dispose();
    this.compositionRange = next;
    this.projectComposition(this.viewport.layout);
  }

  getTargetAtClientPoint(
    point: AlphaClientPoint,
  ): AlphaEditorHitTarget | undefined {
    validateClientPoint(point);
    const bounds = this.element.getBoundingClientRect();
    return this.hitTestViewportPoint(
      point.clientX - bounds.left,
      point.clientY - bounds.top,
    );
  }

  getNearestTargetAtClientPoint(point: AlphaClientPoint): AlphaEditorHitTarget | undefined {
    validateClientPoint(point);
    const layout = this.viewport.layout;
    if (
      layout.viewportSize.width === 0 ||
      layout.viewportSize.height === 0
    ) {
      return undefined;
    }
    const bounds = this.element.getBoundingClientRect();
    return this.hitTestViewportPoint(
      clamp(point.clientX - bounds.left, 0, layout.viewportSize.width - 0.5),
      clamp(point.clientY - bounds.top, 0, layout.viewportSize.height - 0.5),
    );
  }

  private hitTestViewportPoint(left: number, top: number): AlphaEditorHitTarget | undefined {
    return hitTestAlphaEditorPoint(
      this.model,
      this.viewport.layout,
      { left, top },
      {
        gutterWidth: this.gutterWidth,
        textLeft: this.textLeft,
      },
      this.textMeasurer,
    );
  }

  private get measuredContentWidth(): number {
    return Math.ceil(
      this.gutterWidth +
      this.lineWidths.maximumLineWidth +
      this.textMeasurer.horizontalPadding,
    );
  }

  private get gutterWidth(): number {
    if (this.presentation === "embedded") return 0;
    const digitCount = String(this.model.lineCount).length;
    return Math.ceil(
      this.textMeasurer.measureLineWidth("9".repeat(digitCount)) +
      GUTTER_HORIZONTAL_PADDING,
    );
  }

  private get textLeft(): number {
    return this.gutterWidth + this.textMeasurer.contentLeftPadding;
  }

  private project(layout: EditorViewportLayout): void {
    this.element.style.setProperty(
      "--alpha-editor-gutter-width",
      `${this.gutterWidth}px`,
    );
    this.contentElement.style.width = `${layout.contentSize.width}px`;
    this.contentElement.style.height = `${layout.contentSize.height}px`;
    this.linesElement.style.transform =
      `translate3d(0, ${layout.renderTop}px, 0)`;
    this.reconcileLines(layout);
    this.projectDecorations(layout);
    this.projectComposition(layout);
    this.projectSelections(layout);
    this.syncScrollPosition(layout);
  }

  private reconcileLines(layout: EditorViewportLayout): void {
    if (
      this.renderedModelVersion === layout.modelVersion &&
      this.renderedLineHeight === layout.lineHeight &&
      lineRangesEqual(this.renderedRange, layout.renderLines)
    ) return;

    const ownerDocument = this.element.ownerDocument;
    const semanticTokens = this.resolveSemanticTokenRange(layout.renderLines);
    const fragment = ownerDocument.createDocumentFragment();
    const next = new Map<number, AlphaRenderedLine>();
    for (
      let lineIndex = layout.renderLines.startLineIndex;
      lineIndex < layout.renderLines.endLineIndexExclusive;
      lineIndex++
    ) {
      const existing = this.renderedLines.get(lineIndex);
      const line = existing ?? createAlphaRenderedLine(ownerDocument, lineIndex);
      if (!existing) {
        line.numberElement.textContent = String(lineIndex + 1);
      }
      if (!existing || this.renderedModelVersion !== layout.modelVersion) {
        this.projectLineText(line, lineIndex, semanticTokens.get(lineIndex) ?? []);
      }
      if (!existing || this.renderedLineHeight !== layout.lineHeight) {
        line.element.style.height = `${layout.lineHeight}px`;
        line.element.style.lineHeight = `${layout.lineHeight}px`;
      }
      next.set(lineIndex, line);
      fragment.append(line.element);
    }
    reset(this.linesElement, fragment);
    this.renderedLines = next;
    this.renderedRange = layout.renderLines;
    this.renderedModelVersion = layout.modelVersion;
    this.renderedLineHeight = layout.lineHeight;
  }

  private projectVisibleLineText(): void {
    const semanticTokens = this.resolveSemanticTokenRange(this.renderedRange);
    for (const [lineIndex, line] of this.renderedLines) {
      if (lineIndex < this.model.lineCount) this.projectLineText(line, lineIndex, semanticTokens.get(lineIndex) ?? []);
    }
  }

  private projectLineText(line: AlphaRenderedLine, lineIndex: number, tokens: readonly AlphaResolvedSemanticToken[]): void {
    projectAlphaSemanticTokenLine(
      line.textElement,
      this.model.getLineContent(lineIndex),
      tokens,
    );
  }

  private resolveSemanticTokenRange(range: EditorLineRange): ReadonlyMap<number, readonly AlphaResolvedSemanticToken[]> {
    const source = this.semanticTokenSource;
    if (!source) return new Map();
    const tokens = new Map<number, readonly AlphaResolvedSemanticToken[]>();
    const endLineIndex = Math.min(range.endLineIndexExclusive, this.model.lineCount);
    for (let lineIndex = range.startLineIndex; lineIndex < endLineIndex; lineIndex += 1) {
      tokens.set(lineIndex, source.getLineTokens(lineIndex));
    }
    return tokens;
  }

  private projectSelections(layout: EditorViewportLayout): void {
    projectAlphaSelectionOverlays(this.overlayContext(layout), this.selectionController);
  }

  private projectDecorations(layout: EditorViewportLayout): void {
    projectAlphaDecorationOverlays(this.overlayContext(layout), this.resolvedDecorations);
  }

  private projectComposition(layout: EditorViewportLayout): void {
    projectAlphaCompositionOverlay(this.overlayContext(layout), this.compositionRange?.range);
  }

  private overlayContext(layout: EditorViewportLayout): AlphaViewportOverlayContext {
    return {
      ownerDocument: this.element.ownerDocument,
      model: this.model,
      renderedLines: this.renderedLines,
      renderLines: layout.renderLines,
      textLeft: this.textLeft,
      textMeasurer: this.textMeasurer,
    };
  }

  private get resolvedDecorations(): readonly AlphaResolvedDecoration[] {
    return this.decorationSources.flatMap(
      source => this.decorationSnapshots.get(source) ?? [],
    );
  }

  private syncScrollPosition(layout: EditorViewportLayout): void {
    if (this.element.scrollLeft !== layout.scrollPosition.left) {
      this.element.scrollLeft = layout.scrollPosition.left;
    }
    if (this.element.scrollTop !== layout.scrollPosition.top) {
      this.element.scrollTop = layout.scrollPosition.top;
    }
  }
}

function validateClientPoint(point: AlphaClientPoint): void {
  if (
    !point ||
    !Number.isFinite(point.clientX) ||
    !Number.isFinite(point.clientY)
  ) {
    throw new RangeError(
      "Alpha client point must contain finite coordinates",
    );
  }
}

function clamp(value: number, minimum: number, maximum: number): number {
  return Math.min(Math.max(value, minimum), maximum);
}

function lineRangesEqual(left: EditorLineRange, right: EditorLineRange): boolean {
  return left.startLineIndex === right.startLineIndex &&
    left.endLineIndexExclusive === right.endLineIndexExclusive;
}
