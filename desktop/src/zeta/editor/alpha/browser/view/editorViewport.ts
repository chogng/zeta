import "../media/editorViewport.css";
import { addDisposableListener, reset } from "../../../../base/browser/dom.js";
import { getClientArea } from "../../../../base/browser/geometry.js";
import { runWhenWindowIdle } from "../../../../base/browser/scheduler.js";
import { type Event } from "../../../../base/common/event.js";
import { type ISize } from "../../../../base/common/layout.js";
import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import { type EditorSelectionController } from "../../common/cursor/editorSelectionController.js";
import { resolveEditorIndentationOptions, type EditorIndentationOptions, type ResolvedEditorIndentationOptions } from "../../contrib/indentation/common/indentation.js";
import { type EditorHiddenRangeModel } from "../../contrib/folding/browser/hiddenRangeModel.js";
import { type EditorFoldingModel } from "../../contrib/folding/browser/foldingModel.js";
import { TextPosition, type TextRange } from "../../common/core/text.js";
import { type TextModel } from "../../common/model/textModel.js";
import { TrackedRangeStickiness, type TrackedRange } from "../../common/model/trackedRange.js";
import { type EditorVisualLineProjection } from "../../common/viewModel/modelLineProjection.js";
import { type EditorLineRange, type EditorScrollPosition, type EditorViewportChange, type EditorViewportLayout, EditorViewportModel } from "../../common/viewLayout/editorViewportModel.js";
import { type DecorationSource, type ResolvedDecoration } from "./decorationPresentation.js";
import { DecorationLineIndex } from "./decorationLineIndex.js";
import { createAlphaDiagnosticOverviewMarkers } from "../../contrib/gotoError/browser/diagnosticOverviewRuler.js";
import { DomTextMeasurer, type TextMeasurer } from "./fontMetrics.js";
import { FoldingDecorationProvider } from "../../contrib/folding/browser/foldingDecorations.js";
import { type BracketColorizationSource } from "../../contrib/bracketMatching/browser/bracketColorizationPresentation.js";
import { LineWidthIndex } from "../../contrib/longLinesHelper/browser/longLinesHelper.js";
import { createAlphaIndentationGuides } from "../../contrib/indentation/browser/indentation.js";
import { GpuMinimapRenderer } from "../../contrib/gpu/browser/gpuRenderer.js";
import { MinimapNavigationController } from "./minimapNavigationController.js";
import { createAlphaMinimapRows } from "./minimapProjection.js";
import { type ClientPoint, type EditorHitTarget, EditorHitTargetKind, hitTestAlphaVisualEditorPoint } from "./pointerHitTest.js";
import { getAlphaDomTextCaretLeft, getAlphaDomTextOffsetAtClientPoint } from "./domTextGeometry.js";
import { createAlphaRenderedLine, type RenderedLine } from "./renderedLine.js";
import { projectAlphaSemanticTokenLine, type ResolvedSemanticToken, type SemanticTokenSource } from "../../browser/view/semanticTokenPresentation.js";
import { type BracketColorizationSpan } from "../../browser/view/semanticTokenPresentation.js";
import { projectAlphaCompositionOverlay, projectAlphaDecorationOverlays, projectAlphaSelectionOverlays, type ActiveLineHighlight, type ViewportOverlayContext } from "./viewportOverlayPresentation.js";
import { EditorLineWrapping, VisualLineProjection } from "./visualLineProjection.js";
import { VisibleLineProjection } from "./visibleLineProjection.js";
import { getTextGraphemeBoundaries } from "../../common/core/textSegmentation.js";

const GUTTER_HORIZONTAL_PADDING = 16;
const OVERVIEW_RULER_WIDTH = 6;
const MINIMAP_WIDTH = 56;

export type EditorViewportPresentation = "document" | "embedded";

/** Chooses which component renders the visible focus outline for an Alpha viewport. */
export type EditorFocusOutlineOwner = "editor" | "host";

/** Controls whether the viewport projects current-line presentation DOM. */
export type EditorActiveLineHighlight = ActiveLineHighlight;

/** Space reserved around the editor's projected text rows. */
export interface EditorViewportPadding {
  readonly top: number;
  readonly right: number;
  readonly bottom: number;
  readonly left: number;
}

export enum EditorMinimap {
  On = "on",
  Off = "off",
}

/** Controls the browser paragraph direction used to shape Alpha's rendered text. */
export enum EditorTextDirection {
  Auto = "auto",
  LeftToRight = "ltr",
  RightToLeft = "rtl",
}

export interface EditorViewportOptions {
  readonly container: HTMLElement;
  readonly model: TextModel;
  readonly lineHeight: number;
  readonly padding?: EditorViewportPadding;
  readonly overscanLineCount?: number;
  readonly ariaLabel?: string;
  readonly textMeasurer?: TextMeasurer;
  readonly selectionController?: EditorSelectionController;
  readonly decorationSources?: readonly DecorationSource[];
  readonly semanticTokenSource?: SemanticTokenSource;
  readonly bracketColorizationSource?: BracketColorizationSource;
  readonly foldingModel?: EditorFoldingModel;
  readonly hiddenRangeModel?: EditorHiddenRangeModel;
  readonly presentation?: EditorViewportPresentation;
  /** `host` delegates the visible focus outline to the viewport's direct host. */
  readonly focusOutlineOwner?: EditorFocusOutlineOwner;
  /** `off` omits current-line presentation while preserving selections and carets. */
  readonly activeLineHighlight?: EditorActiveLineHighlight;
  readonly lineWrapping?: EditorLineWrapping;
  readonly minimap?: EditorMinimap;
  readonly indentation?: EditorIndentationOptions;
  /** Browser text-direction input; automatic direction is the default. */
  readonly textDirection?: EditorTextDirection;
}

export interface EditorContentPosition {
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
export class EditorViewport extends DisposableOwner {
  readonly element: HTMLDivElement;
  readonly onDidChangeLayout: Event<EditorViewportChange>;
  private readonly model: TextModel;
  private readonly viewport: EditorViewportModel;
  private readonly contentElement: HTMLDivElement;
  private readonly linesElement: HTMLDivElement;
  private readonly textMetricsElement: HTMLSpanElement;
  private readonly accessibilityStatusElement: HTMLDivElement;
  private readonly overviewRulerElement: HTMLDivElement;
  private readonly minimapElement: HTMLDivElement;
  private readonly minimapCanvasElement: HTMLCanvasElement;
  private readonly minimapViewportElement: HTMLDivElement;
  private readonly minimapGpuRenderer: GpuMinimapRenderer | undefined;
  private readonly textMeasurer: TextMeasurer;
  private readonly lineWidths: LineWidthIndex;
  private readonly visualLineProjection: VisualLineProjection;
  private readonly visibleLineProjection: VisibleLineProjection;
  private readonly foldingDecorations: FoldingDecorationProvider;
  private readonly selectionController: EditorSelectionController | undefined;
  private readonly decorationSources: readonly DecorationSource[];
  private readonly semanticTokenSource: SemanticTokenSource | undefined;
  private readonly bracketColorizationSource: BracketColorizationSource | undefined;
  private readonly presentation: EditorViewportPresentation;
  private readonly focusOutlineOwner: EditorFocusOutlineOwner;
  private readonly activeLineHighlight: EditorActiveLineHighlight;
  private readonly padding: EditorViewportPadding;
  private readonly indentation: ResolvedEditorIndentationOptions;
  private readonly decorationSnapshots =
    new Map<DecorationSource, DecorationSource["decorations"]>();
  private decorationLineIndex = new DecorationLineIndex([]);
  private renderedLines = new Map<number, RenderedLine>();
  private renderedRange: EditorLineRange = {
    startLineIndex: 0,
    endLineIndexExclusive: 0,
  };
  private renderedModelVersion = -1;
  private renderedLineHeight = -1;
  private renderedVisualProjectionRevision = -1;
  private overviewRevision = 0;
  private renderedOverviewRevision = -1;
  private minimapRevision = 0;
  private renderedMinimapRevision = -1;
  private readonly minimap: EditorMinimap;
  private readonly textDirection: EditorTextDirection;
  private softWrapping: boolean;
  private compositionRange: TrackedRange | undefined;

  constructor(options: EditorViewportOptions) {
    super();
    const ownerDocument = options.container.ownerDocument;
    this.model = options.model;
    this.element = ownerDocument.createElement("div");
    this.contentElement = ownerDocument.createElement("div");
    this.linesElement = ownerDocument.createElement("div");
    this.textMetricsElement = ownerDocument.createElement("span");
    this.accessibilityStatusElement = ownerDocument.createElement("div");
    this.overviewRulerElement = ownerDocument.createElement("div");
    this.minimapElement = ownerDocument.createElement("div");
    this.minimapCanvasElement = ownerDocument.createElement("canvas");
    this.minimapViewportElement = ownerDocument.createElement("div");
    this.selectionController = options.selectionController;
    this.semanticTokenSource = options.semanticTokenSource;
    this.bracketColorizationSource = options.bracketColorizationSource;
    this.presentation = options.presentation ?? "document";
    this.focusOutlineOwner = options.focusOutlineOwner ?? "editor";
    this.activeLineHighlight = options.activeLineHighlight ?? (this.presentation === "embedded" ? "off" : "on");
    this.padding = resolveEditorViewportPadding(options.padding);
    this.minimap = options.minimap ?? (this.presentation === "document" ? EditorMinimap.On : EditorMinimap.Off);
    this.textDirection = options.textDirection ?? EditorTextDirection.Auto;
    this.softWrapping = options.lineWrapping === EditorLineWrapping.On;
    try {
      this.indentation = resolveEditorIndentationOptions(options.indentation);
      if (!Object.values(EditorMinimap).includes(this.minimap)) {
        throw new TypeError("Unknown Alpha editor minimap mode");
      }
      if (!Object.values(EditorTextDirection).includes(this.textDirection)) {
        throw new TypeError("Unknown Alpha editor text direction");
      }
      if (this.focusOutlineOwner !== "editor" && this.focusOutlineOwner !== "host") {
        throw new TypeError("Unknown Alpha editor focus outline owner");
      }
      if (this.activeLineHighlight !== "on" && this.activeLineHighlight !== "off") {
        throw new TypeError("Unknown Alpha editor active-line highlight");
      }
      if (this.selectionController && this.selectionController.textModel !== this.model) {
        throw new TypeError(
          "Alpha viewport and selection controller must share one text model",
        );
      }
      if (this.semanticTokenSource && this.semanticTokenSource.textModel !== this.model) {
        throw new TypeError("Alpha viewport and semantic token source must share one text model");
      }
      if (this.bracketColorizationSource && this.bracketColorizationSource.textModel !== this.model) {
        throw new TypeError("Alpha viewport and bracket colorization source must share one text model");
      }
      if (options.foldingModel && options.foldingModel.model !== this.model) {
        throw new TypeError("Alpha viewport and folding model must share one text model");
      }
      if (options.hiddenRangeModel && options.hiddenRangeModel.model !== this.model) {
        throw new TypeError("Alpha viewport and hidden range model must share one text model");
      }
      if (options.foldingModel && !options.hiddenRangeModel) {
        throw new TypeError("Alpha viewport folding requires a hidden range model");
      }
    } catch (error) {
      this.dispose();
      throw error;
    }
    this.foldingDecorations = this.own(new FoldingDecorationProvider(options.foldingModel));
    this.decorationSources = Object.freeze([
      ...(options.decorationSources ?? []),
    ]);

    this.element.className = "zeta-alpha-editor";
    this.element.classList.add(`zeta-alpha-editor-${this.presentation}`);
    this.element.classList.add(`zeta-alpha-editor-focus-owner-${this.focusOutlineOwner}`);
    this.element.classList.add(`zeta-alpha-editor-direction-${this.textDirection}`);
    this.element.style.setProperty("--alpha-editor-padding-left", `${this.padding.left}px`);
    this.element.style.setProperty("--alpha-editor-padding-right", `${this.padding.right}px`);
    this.element.dir = this.textDirection;
    this.element.classList.toggle("word-wrapped", this.softWrapping);
    this.element.tabIndex = 0;
    this.element.setAttribute("role", "region");
    this.element.setAttribute("aria-label", options.ariaLabel ?? "Alpha editor");
    this.contentElement.className = "zeta-alpha-editor-content";
    this.linesElement.className = "zeta-alpha-editor-lines";
    this.textMetricsElement.className =
      "zeta-alpha-editor-text-metrics";
    this.textMetricsElement.setAttribute("aria-hidden", "true");
    this.accessibilityStatusElement.className = "zeta-alpha-editor-accessibility-status";
    this.accessibilityStatusElement.setAttribute("aria-live", "polite");
    this.accessibilityStatusElement.setAttribute("aria-atomic", "true");
    this.overviewRulerElement.className = "zeta-alpha-editor-overview-ruler";
    this.overviewRulerElement.setAttribute("aria-hidden", "true");
    this.minimapElement.className = "zeta-alpha-editor-minimap";
    this.minimapElement.hidden = this.minimap === EditorMinimap.Off;
    this.minimapElement.setAttribute("aria-hidden", "true");
    this.minimapCanvasElement.className = "zeta-alpha-editor-minimap-gpu";
    this.minimapCanvasElement.setAttribute("aria-hidden", "true");
    this.minimapViewportElement.className = "zeta-alpha-editor-minimap-viewport";
    this.minimapElement.append(this.minimapCanvasElement, this.minimapViewportElement);
    this.contentElement.append(this.linesElement);
    this.element.append(this.contentElement, this.overviewRulerElement, this.minimapElement, this.textMetricsElement, this.accessibilityStatusElement);
    options.container.append(this.element);
    this.defer(() => this.element.remove());
    this.defer(() => this.compositionRange?.dispose());
    this.minimapGpuRenderer = this.minimap === EditorMinimap.On
      ? GpuMinimapRenderer.tryCreate(this.minimapCanvasElement)
      : undefined;
    this.defer(() => this.minimapGpuRenderer?.dispose());
    this.textMeasurer =
      options.textMeasurer ??
      new DomTextMeasurer(this.textMetricsElement);
    this.lineWidths = this.own(new LineWidthIndex(
      this.model,
      this.textMeasurer,
      {
        initialMeasurement: {
          schedule: callback => runWhenWindowIdle(
            ownerDocument.defaultView!,
            () => callback(),
            { timeoutMs: 250 },
          ),
        },
      },
    ));
    this.visualLineProjection = this.own(new VisualLineProjection(
      this.model,
      this.textMeasurer,
      {
        wrapping: options.lineWrapping,
        initialWrappingMeasurement: {
          schedule: callback => runWhenWindowIdle(
            ownerDocument.defaultView!,
            () => callback(),
            { timeoutMs: 250 },
          ),
        },
      },
    ));
    this.visibleLineProjection = this.own(new VisibleLineProjection(
      this.visualLineProjection,
      options.hiddenRangeModel,
    ));
    const viewport = this.own(new EditorViewportModel(this.model, {
      lineHeight: options.lineHeight,
      overscanLineCount: options.overscanLineCount,
      lineSource: this.visibleLineProjection.lineSource,
      padding: { top: this.padding.top, bottom: this.padding.bottom },
    }));
    this.viewport = viewport;
    this.onDidChangeLayout = viewport.onDidChange;
    this.own(this.visibleLineProjection.onDidChange(() => this.project(viewport.layout)));
    this.own(this.foldingDecorations.onDidChange(() => this.project(viewport.layout)));
    viewport.setContentWidth(this.measuredContentWidth);

    this.own(viewport.onDidChange(({ layout }) => this.project(layout)));
    this.own(this.lineWidths.onDidChange(() => {
      viewport.setContentWidth(this.measuredContentWidth);
    }));
    this.own(addDisposableListener(this.element, "scroll", () => {
      const layout = viewport.setScrollPosition({
        left: this.element.scrollLeft,
        top: this.element.scrollTop,
      });
      this.syncScrollPosition(layout);
    }));
    this.own(new MinimapNavigationController(
      this.minimapElement,
      ownerDocument,
      () => this.viewport.layout,
      position => this.scrollTo(position),
    ));
    this.own(addDisposableListener<globalThis.Event>(this.minimapCanvasElement, "webglcontextlost", event => {
      event.preventDefault();
      this.minimapGpuRenderer?.disable();
      this.renderedMinimapRevision = -1;
      this.projectMinimap(this.viewport.layout);
    }));
    this.own(this.model.onDidChange(change => {
      this.lineWidths.applyModelChange(change);
      this.overviewRevision += 1;
      this.minimapRevision += 1;
      if (this.softWrapping) this.updateWrapWidth(this.viewport.layout.viewportSize.width);
      viewport.setContentWidth(this.measuredContentWidth);
    }));
    if (this.selectionController) {
      this.own(this.selectionController.onDidChange(() => {
        this.projectSelections(viewport.layout);
        this.updateAccessibilityStatus();
      }));
      this.updateAccessibilityStatus();
    }
    for (const source of this.decorationSources) {
      this.decorationSnapshots.set(source, source.decorations);
      this.own(source.onDidChange(() => {
        this.decorationSnapshots.set(source, source.decorations);
        this.rebuildDecorationLineIndex();
        this.projectDecorations(viewport.layout);
        this.projectOverviewRuler(viewport.layout);
        this.projectMinimap(viewport.layout);
      }));
    }
    this.rebuildDecorationLineIndex();
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
      const observer = new ResizeObserverConstructor(() => this.layout());
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

  get lineWrapping(): EditorLineWrapping {
    return this.softWrapping
      ? EditorLineWrapping.On
      : EditorLineWrapping.Off;
  }

  /** Returns the browser paragraph direction used by the text projection. */
  get editorTextDirection(): EditorTextDirection {
    return this.textDirection;
  }

  /** Changes only this viewport's visual row projection; document text is unaffected. */
  setLineWrapping(lineWrapping: EditorLineWrapping): EditorViewportLayout {
    if (!Object.values(EditorLineWrapping).includes(lineWrapping)) {
      throw new TypeError("Unknown Alpha editor line wrapping mode");
    }
    const nextSoftWrapping = lineWrapping === EditorLineWrapping.On;
    if (nextSoftWrapping === this.softWrapping) return this.viewport.layout;
    this.softWrapping = nextSoftWrapping;
    if (nextSoftWrapping) this.updateWrapWidth(this.viewport.layout.viewportSize.width);
    this.visualLineProjection.setWrapping(lineWrapping);
    this.element.classList.toggle("word-wrapped", nextSoftWrapping);
    const layout = this.viewport.setContentWidth(this.measuredContentWidth);
    this.project(layout);
    return layout;
  }

  /** Returns the current measured visual-row mapping for browser interactions. */
  getVisualLineProjection(): EditorVisualLineProjection {
    return this.visualProjection;
  }

  /** Measures text with the same font metrics used by the rendered viewport. */
  measureTextWidth(text: string): number {
    return this.textMeasurer.measureLineWidth(text);
  }

  layout(size: ISize = getClientArea(this.element)): EditorViewportLayout {
    this.refreshFontMetrics();
    if (this.softWrapping) this.updateWrapWidth(size.width);
    const layout = this.viewport.setViewportSize(size);
    this.project(layout);
    return layout;
  }

  /** Announces one editor status message through the viewport's live region. */
  announceAccessibilityStatus(message: string): void {
    if (typeof message !== "string" || message.trim().length === 0) {
      throw new TypeError("Alpha accessibility status must be a non-empty string");
    }
    this.accessibilityStatusElement.textContent = message.trim();
  }

  refreshFontMetrics(): EditorViewportLayout {
    if (!this.textMeasurer.refresh()) return this.viewport.layout;
    this.lineWidths.refresh();
    if (this.softWrapping) this.updateWrapWidth(this.viewport.layout.viewportSize.width);
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
    const visualProjection = this.visualProjection;
    const visualLineIndex = visualProjection.visualLineIndexAt(position);
    const visualLine = visualProjection.lineAt(visualLineIndex)!;
    const lineTop = this.padding.top + visualLineIndex * layout.lineHeight;
    const lineBottom = lineTop + layout.lineHeight;
    let top = layout.scrollPosition.top;
    if (lineTop < top) {
      top = lineTop;
    } else if (lineBottom > top + layout.viewportSize.height) {
      top = lineBottom - layout.viewportSize.height;
    }

    const line = this.model.getLineContent(visualLine.logicalLineIndex);
    const domCaretLeft = this.domCaretLeft(visualLineIndex, position.columnIndex - visualLine.startColumn);
    const caretLeft = domCaretLeft ?? (this.textLeft +
      this.textMeasurer.measureLineWidth(line.slice(visualLine.startColumn, position.columnIndex)));
    const caretRight = caretLeft + Math.max(
      1,
      this.textMeasurer.measureLineWidth(" "),
    );
    let left = this.softWrapping ? 0 : layout.scrollPosition.left;
    if (caretLeft < left + this.textLeft) {
      left = caretLeft - this.textLeft;
    } else if (caretRight > left + layout.viewportSize.width) {
      left = caretRight - layout.viewportSize.width;
    }
    return this.scrollTo({ left, top });
  }

  getPositionContentCoordinates(position: TextPosition): EditorContentPosition {
    this.model.offsetAt(position);
    const visualProjection = this.visualProjection;
    const visualLineIndex = visualProjection.visualLineIndexAt(position);
    const visualLine = visualProjection.lineAt(visualLineIndex)!;
    const domCaretLeft = this.domCaretLeft(visualLineIndex, position.columnIndex - visualLine.startColumn);
    return Object.freeze({
      left: domCaretLeft ?? (this.textLeft + this.textMeasurer.measureLineWidth(
        this.model.getLineContent(visualLine.logicalLineIndex).slice(visualLine.startColumn, position.columnIndex),
      )),
      top: this.padding.top + visualLineIndex * this.viewport.layout.lineHeight,
      height: this.viewport.layout.lineHeight,
    });
  }

  /** Returns a browser-shaped x-coordinate for one rendered visual cursor, when available. */
  getVisualHorizontalOffset(position: TextPosition): number | undefined {
    this.model.offsetAt(position);
    const visualLineIndex = this.visualProjection.visualLineIndexAt(position);
    const visualLine = this.visualProjection.lineAt(visualLineIndex);
    return visualLine
      ? this.domCaretLeft(visualLineIndex, position.columnIndex - visualLine.startColumn)
      : undefined;
  }

  /** Resolves the nearest browser-shaped cursor on one currently rendered visual line. */
  getNearestPositionAtVisualHorizontalOffset(visualLineIndex: number, horizontalOffset: number): TextPosition | undefined {
    if (!Number.isFinite(horizontalOffset)) throw new RangeError("Alpha visual cursor horizontal offset must be finite");
    if (this.textDirection === EditorTextDirection.LeftToRight) return undefined;
    const visualLine = this.visualProjection.lineAt(visualLineIndex);
    const line = this.renderedLines.get(visualLineIndex);
    if (!visualLine || !line) return undefined;
    const text = this.model.getLineContent(visualLine.logicalLineIndex).slice(visualLine.startColumn, visualLine.endColumn);
    if (line.textElement.textContent?.length !== text.length) return undefined;
    let nearestColumn: number | undefined;
    let nearestDistance = Number.POSITIVE_INFINITY;
    for (const column of getTextGraphemeBoundaries(text)) {
      const left = getAlphaDomTextCaretLeft(line.textElement, column, line.element);
      if (left === undefined) return undefined;
      const distance = Math.abs(left - horizontalOffset);
      if (distance < nearestDistance) {
        nearestColumn = column;
        nearestDistance = distance;
      }
    }
    return nearestColumn === undefined
      ? undefined
      : TextPosition.at(visualLine.logicalLineIndex, visualLine.startColumn + nearestColumn);
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
    point: ClientPoint,
  ): EditorHitTarget | undefined {
    validateClientPoint(point);
    const domTarget = this.textDirection === EditorTextDirection.LeftToRight
      ? undefined
      : this.getDomTargetAtClientPoint(point);
    if (domTarget) return domTarget;
    const bounds = this.element.getBoundingClientRect();
    return this.hitTestViewportPoint(
      point.clientX - bounds.left,
      point.clientY - bounds.top,
    );
  }

  getNearestTargetAtClientPoint(point: ClientPoint): EditorHitTarget | undefined {
    validateClientPoint(point);
    const domTarget = this.textDirection === EditorTextDirection.LeftToRight
      ? undefined
      : this.getDomTargetAtClientPoint(point);
    if (domTarget) return domTarget;
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

  private hitTestViewportPoint(left: number, top: number): EditorHitTarget | undefined {
    return hitTestAlphaVisualEditorPoint(
      this.model,
      this.visualProjection,
      this.viewport.layout,
      { left, top },
      {
        gutterWidth: this.gutterWidth,
        textLeft: this.textLeft,
        paddingTop: this.padding.top,
      },
      this.textMeasurer,
    );
  }

  private getDomTargetAtClientPoint(point: ClientPoint): EditorHitTarget | undefined {
    for (const [visualLineIndex, renderedLine] of this.renderedLines) {
      const offset = getAlphaDomTextOffsetAtClientPoint(
        renderedLine.textElement,
        point.clientX,
        point.clientY,
      );
      if (offset === undefined) continue;
      const visualLine = this.visualProjection.lineAt(visualLineIndex);
      if (!visualLine) continue;
      return Object.freeze({
        kind: EditorHitTargetKind.Text,
        position: TextPosition.at(visualLine.logicalLineIndex, visualLine.startColumn + offset),
      });
    }
    return undefined;
  }

  private domCaretLeft(visualLineIndex: number, offset: number): number | undefined {
    if (this.textDirection === EditorTextDirection.LeftToRight) return undefined;
    const line = this.renderedLines.get(visualLineIndex);
    return line && Number.isSafeInteger(offset) && offset >= 0 && offset <= line.textElement.textContent?.length
      ? getAlphaDomTextCaretLeft(line.textElement, offset, line.element)
      : undefined;
  }

  private get measuredContentWidth(): number {
    if (this.softWrapping) return 0;
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

  private updateWrapWidth(viewportWidth: number): void {
    this.visualLineProjection.setWrapWidth(Math.max(
      0,
      viewportWidth - this.gutterWidth - this.textMeasurer.horizontalPadding,
    ));
  }

  private project(layout: EditorViewportLayout): void {
    this.element.classList.toggle("horizontally-scrollable", layout.maximumScrollPosition.left > 0);
    this.element.classList.toggle("vertically-scrollable", layout.maximumScrollPosition.top > 0);
    this.element.style.setProperty(
      "--alpha-editor-gutter-width",
      `${this.gutterWidth}px`,
    );
    this.contentElement.style.width = `${layout.contentSize.width}px`;
    this.contentElement.style.height = `${layout.contentSize.height}px`;
    this.linesElement.style.transform =
      `translate3d(0, ${layout.renderTop}px, 0)`;
    this.reconcileLines(layout);
    this.projectIndentationGuides(layout);
    this.projectDecorations(layout);
    this.projectComposition(layout);
    this.projectSelections(layout);
    this.projectOverviewRuler(layout);
    this.projectMinimap(layout);
    this.syncScrollPosition(layout);
  }

  private reconcileLines(layout: EditorViewportLayout): void {
    const visualProjection = this.visualProjection;
      const visualProjectionRevision = this.visibleLineProjection.revision;
    if (
      this.renderedModelVersion === layout.modelVersion &&
      this.renderedLineHeight === layout.lineHeight &&
      this.renderedVisualProjectionRevision === visualProjectionRevision &&
      lineRangesEqual(this.renderedRange, layout.renderLines)
    ) return;

    const ownerDocument = this.element.ownerDocument;
    const semanticTokens = this.resolveSemanticTokenRange(layout.renderLines);
    const fragment = ownerDocument.createDocumentFragment();
    const next = new Map<number, RenderedLine>();
    for (
      let visualLineIndex = layout.renderLines.startLineIndex;
      visualLineIndex < layout.renderLines.endLineIndexExclusive;
      visualLineIndex++
    ) {
      const visualLine = visualProjection.lineAt(visualLineIndex);
      if (!visualLine) throw new Error("Viewport render range exceeds the visual line projection");
      const existing = this.renderedLines.get(visualLineIndex);
      const line = existing ?? createAlphaRenderedLine(ownerDocument, visualLineIndex);
      line.element.dataset.logicalLineIndex = String(visualLine.logicalLineIndex);
      if (!existing || this.renderedVisualProjectionRevision !== visualProjectionRevision) {
        line.numberElement.textContent = visualLine.firstForLogicalLine
          ? String(visualLine.logicalLineIndex + 1)
          : "";
        this.foldingDecorations.project(line.foldingElement, visualLine.logicalLineIndex, visualLine.firstForLogicalLine);
      }
      if (
        !existing ||
        this.renderedModelVersion !== layout.modelVersion ||
        this.renderedVisualProjectionRevision !== visualProjectionRevision
      ) {
        line.textElement.dir = this.textDirection;
        this.projectLineText(line, visualLine, semanticTokens.get(visualLine.logicalLineIndex) ?? []);
      }
      if (!existing || this.renderedLineHeight !== layout.lineHeight) {
        line.element.style.height = `${layout.lineHeight}px`;
        line.element.style.lineHeight = `${layout.lineHeight}px`;
      }
      next.set(visualLineIndex, line);
      fragment.append(line.element);
    }
    reset(this.linesElement, fragment);
    this.renderedLines = next;
    this.renderedRange = layout.renderLines;
    this.renderedModelVersion = layout.modelVersion;
    this.renderedLineHeight = layout.lineHeight;
    this.renderedVisualProjectionRevision = visualProjectionRevision;
  }

  private projectVisibleLineText(): void {
    const semanticTokens = this.resolveSemanticTokenRange(this.renderedRange);
    const visualProjection = this.visualProjection;
    for (const [visualLineIndex, line] of this.renderedLines) {
      const visualLine = visualProjection.lineAt(visualLineIndex);
      if (visualLine) this.projectLineText(line, visualLine, semanticTokens.get(visualLine.logicalLineIndex) ?? []);
    }
  }

  private projectLineText(line: RenderedLine, visualLine: { readonly logicalLineIndex: number; readonly startColumn: number; readonly endColumn: number }, tokens: readonly ResolvedSemanticToken[]): void {
    const fullText = this.model.getLineContent(visualLine.logicalLineIndex);
    const text = fullText.slice(visualLine.startColumn, visualLine.endColumn);
    const brackets = this.bracketColorizationSource?.getLineBrackets(visualLine.logicalLineIndex) ?? [];
    projectAlphaSemanticTokenLine(
      line.textElement,
      text,
      clipSemanticTokens(tokens, visualLine.startColumn, visualLine.endColumn),
      clipBracketColorizations(brackets, visualLine.startColumn, visualLine.endColumn),
    );
  }

  private projectIndentationGuides(layout: EditorViewportLayout): void {
    const visualProjection = this.visualProjection;
    for (const [visualLineIndex, line] of this.renderedLines) {
      const visualLine = visualProjection.lineAt(visualLineIndex);
      line.indentationElement.replaceChildren();
      if (!visualLine?.firstForLogicalLine) continue;
      const text = this.model.getLineContent(visualLine.logicalLineIndex);
      for (const guide of createAlphaIndentationGuides(text, this.indentation.tabSize)) {
        const element = this.element.ownerDocument.createElement("span");
        element.className = "zeta-alpha-editor-indent-guide";
        element.dataset.indentLevel = String(guide.level);
        element.style.left = `${this.textLeft + this.textMeasurer.measureLineWidth(text.slice(0, guide.columnIndex)) - 1}px`;
        line.indentationElement.append(element);
      }
    }
  }


  private resolveSemanticTokenRange(range: EditorLineRange): ReadonlyMap<number, readonly ResolvedSemanticToken[]> {
    const source = this.semanticTokenSource;
    if (!source) return new Map();
    const tokens = new Map<number, readonly ResolvedSemanticToken[]>();
    const projection = this.visualProjection;
    for (let visualLineIndex = range.startLineIndex; visualLineIndex < range.endLineIndexExclusive; visualLineIndex += 1) {
      const visualLine = projection.lineAt(visualLineIndex);
      if (visualLine && !tokens.has(visualLine.logicalLineIndex)) {
        tokens.set(visualLine.logicalLineIndex, source.getLineTokens(visualLine.logicalLineIndex));
      }
    }
    return tokens;
  }

  private projectSelections(layout: EditorViewportLayout): void {
    projectAlphaSelectionOverlays(this.overlayContext(layout), this.selectionController);
  }

  private updateAccessibilityStatus(): void {
    const selectionSet = this.selectionController?.selections;
    const selection = selectionSet?.primary;
    if (!selection || !selectionSet) return;
    const position = selection.active;
    const selectedLength = this.model.offsetAt(selection.range.end) - this.model.offsetAt(selection.range.start);
    if (selectionSet.selections.length > 1) {
      const totalSelectedLength = selectionSet.selections.reduce((length, current) =>
        length + this.model.offsetAt(current.range.end) - this.model.offsetAt(current.range.start), 0);
      const summary = totalSelectedLength === 0
        ? `${selectionSet.selections.length} cursors`
        : `${selectionSet.selections.length} selections, ${totalSelectedLength} characters selected`;
      this.announceAccessibilityStatus(`${summary}; primary at Line ${position.lineIndex + 1}, column ${position.columnIndex + 1}`);
      return;
    }
    this.announceAccessibilityStatus(selectedLength === 0
      ? `Line ${position.lineIndex + 1}, column ${position.columnIndex + 1}`
      : `Line ${position.lineIndex + 1}, column ${position.columnIndex + 1}, ${selectedLength} characters selected`);
  }

  private projectDecorations(layout: EditorViewportLayout): void {
    projectAlphaDecorationOverlays(this.overlayContext(layout), this.resolveVisibleDecorations(layout));
  }

  private projectComposition(layout: EditorViewportLayout): void {
    projectAlphaCompositionOverlay(this.overlayContext(layout), this.compositionRange?.range);
  }

  private overlayContext(layout: EditorViewportLayout): ViewportOverlayContext {
    return {
      ownerDocument: this.element.ownerDocument,
      model: this.model,
      visualLineProjection: this.visualProjection,
      renderedLines: this.renderedLines,
      renderLines: layout.renderLines,
      textLeft: this.textLeft,
      textMeasurer: this.textMeasurer,
      useDomTextGeometry: this.textDirection !== EditorTextDirection.LeftToRight,
      activeLineHighlight: this.activeLineHighlight,
    };
  }

  private rebuildDecorationLineIndex(): void {
    this.decorationLineIndex = new DecorationLineIndex(this.decorationSources.flatMap(
      source => this.decorationSnapshots.get(source) ?? [],
    ));
    this.overviewRevision += 1;
    this.minimapRevision += 1;
  }

  private projectOverviewRuler(layout: EditorViewportLayout): void {
    const rightOffset = this.minimap === EditorMinimap.On ? MINIMAP_WIDTH + 4 : 0;
    this.overviewRulerElement.style.left = `${layout.scrollPosition.left + Math.max(0, layout.viewportSize.width - OVERVIEW_RULER_WIDTH - rightOffset)}px`;
    this.overviewRulerElement.style.top = `${layout.scrollPosition.top}px`;
    this.overviewRulerElement.style.height = `${layout.viewportSize.height}px`;
    if (this.renderedOverviewRevision === this.overviewRevision) return;
    const markers = createAlphaDiagnosticOverviewMarkers(
      this.decorationSources.flatMap(source => this.decorationSnapshots.get(source) ?? []),
      this.model.lineCount,
    );
    const fragment = this.element.ownerDocument.createDocumentFragment();
    for (const marker of markers) {
      const element = this.element.ownerDocument.createElement("span");
      element.className = "zeta-alpha-editor-overview-marker";
      element.classList.add(marker.presentation);
      element.style.top = `${marker.startLineIndex / this.model.lineCount * 100}%`;
      element.style.height = `${Math.max(1, (marker.endLineIndexExclusive - marker.startLineIndex) / this.model.lineCount * 100)}%`;
      if (marker.hoverText !== undefined) element.title = marker.hoverText;
      fragment.append(element);
    }
    reset(this.overviewRulerElement, fragment);
    this.renderedOverviewRevision = this.overviewRevision;
  }

  private projectMinimap(layout: EditorViewportLayout): void {
    if (this.minimap === EditorMinimap.Off) return;
    this.minimapElement.style.left = `${layout.scrollPosition.left + Math.max(0, layout.viewportSize.width - MINIMAP_WIDTH)}px`;
    this.minimapElement.style.top = `${layout.scrollPosition.top}px`;
    this.minimapElement.style.height = `${layout.viewportSize.height}px`;
    this.minimapGpuRenderer?.resize(MINIMAP_WIDTH, layout.viewportSize.height);
    const contentHeight = Math.max(1, layout.contentSize.height);
    this.minimapViewportElement.style.top = `${layout.scrollPosition.top / contentHeight * 100}%`;
    this.minimapViewportElement.style.height = `${Math.max(2, layout.viewportSize.height / contentHeight * 100)}%`;
    if (this.renderedMinimapRevision === this.minimapRevision) return;
    const fragment = this.element.ownerDocument.createDocumentFragment();
    const rows = createAlphaMinimapRows(this.model);
    const gpuRenderer = this.minimapGpuRenderer;
    if (gpuRenderer?.isAvailable) {
      gpuRenderer.setRows(rows, this.model.lineCount);
    } else {
      for (const row of rows) {
        const marker = this.element.ownerDocument.createElement("span");
        marker.className = "zeta-alpha-editor-minimap-row";
        marker.style.top = `${row.startLineIndex / this.model.lineCount * 100}%`;
        marker.style.height = `${Math.max(1, (row.endLineIndexExclusive - row.startLineIndex) / this.model.lineCount * 100)}%`;
        marker.style.width = `${Math.max(8, row.density * 100)}%`;
        fragment.append(marker);
      }
    }
    for (const marker of createAlphaDiagnosticOverviewMarkers(
      this.decorationSources.flatMap(source => this.decorationSnapshots.get(source) ?? []),
      this.model.lineCount,
    )) {
      const element = this.element.ownerDocument.createElement("span");
      element.className = "zeta-alpha-editor-minimap-diagnostic-marker";
      element.classList.add(marker.presentation);
      element.style.top = `${marker.startLineIndex / this.model.lineCount * 100}%`;
      element.style.height = `${Math.max(1, (marker.endLineIndexExclusive - marker.startLineIndex) / this.model.lineCount * 100)}%`;
      if (marker.hoverText !== undefined) element.title = marker.hoverText;
      fragment.append(element);
    }
    this.minimapElement.replaceChildren(this.minimapCanvasElement, fragment, this.minimapViewportElement);
    this.renderedMinimapRevision = this.minimapRevision;
  }

  private resolveVisibleDecorations(layout: EditorViewportLayout): readonly ResolvedDecoration[] {
    const projection = this.visualProjection;
    let minimumLogicalLineIndex = Number.POSITIVE_INFINITY;
    let maximumLogicalLineIndex = -1;
    for (let visualLineIndex = layout.renderLines.startLineIndex; visualLineIndex < layout.renderLines.endLineIndexExclusive; visualLineIndex += 1) {
      const visualLine = projection.lineAt(visualLineIndex);
      if (!visualLine) continue;
      minimumLogicalLineIndex = Math.min(minimumLogicalLineIndex, visualLine.logicalLineIndex);
      maximumLogicalLineIndex = Math.max(maximumLogicalLineIndex, visualLine.logicalLineIndex);
    }
    return maximumLogicalLineIndex < 0
      ? []
      : this.decorationLineIndex.getIntersectingLines(minimumLogicalLineIndex, maximumLogicalLineIndex);
  }

  private get visualProjection() {
    return this.visibleLineProjection.ensureCurrent();
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

function validateClientPoint(point: ClientPoint): void {
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

function resolveEditorViewportPadding(padding: EditorViewportPadding | undefined): EditorViewportPadding {
  return Object.freeze({
    top: nonNegativePaddingValue(padding?.top ?? 0, "top"),
    right: nonNegativePaddingValue(padding?.right ?? 12, "right"),
    bottom: nonNegativePaddingValue(padding?.bottom ?? 0, "bottom"),
    left: nonNegativePaddingValue(padding?.left ?? 12, "left"),
  });
}

function nonNegativePaddingValue(value: number, side: keyof EditorViewportPadding): number {
  if (!Number.isFinite(value) || value < 0) {
    throw new RangeError(`Alpha editor padding.${side} must be non-negative and finite`);
  }
  return value;
}

function clamp(value: number, minimum: number, maximum: number): number {
  return Math.min(Math.max(value, minimum), maximum);
}

function lineRangesEqual(left: EditorLineRange, right: EditorLineRange): boolean {
  return left.startLineIndex === right.startLineIndex &&
    left.endLineIndexExclusive === right.endLineIndexExclusive;
}

function clipSemanticTokens(tokens: readonly ResolvedSemanticToken[], startColumn: number, endColumn: number): readonly ResolvedSemanticToken[] {
  return Object.freeze(tokens.flatMap(token => {
    const start = Math.max(token.startColumn, startColumn);
    const end = Math.min(token.endColumn, endColumn);
    if (end <= start) return [];
    return [Object.freeze({
      startColumn: start - startColumn,
      endColumn: end - startColumn,
      presentation: token.presentation,
      ...(token.modifiers && token.modifiers.length > 0 ? { modifiers: token.modifiers } : {}),
    })];
  }));
}

function clipBracketColorizations(brackets: readonly BracketColorizationSpan[], startColumn: number, endColumn: number): readonly BracketColorizationSpan[] {
  return Object.freeze(brackets.flatMap(bracket => {
    const start = Math.max(bracket.startColumn, startColumn);
    const end = Math.min(bracket.endColumn, endColumn);
    if (end <= start) return [];
    return [Object.freeze({ startColumn: start - startColumn, endColumn: end - startColumn, level: bracket.level })];
  }));
}
