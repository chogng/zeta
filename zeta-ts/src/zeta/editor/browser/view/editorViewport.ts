import "../media/editorViewport.css";
import { addDisposableListener, h } from "../../../base/browser/dom.js";
import { FastDomNode } from "../../../base/browser/fastDomNode.js";
import { getClientArea } from "../../../base/browser/geometry.js";
import { observeResize } from "../../../base/browser/observer.js";
import { runWhenWindowIdle } from "../../../base/browser/scheduler.js";
import { type Event } from "../../../base/common/event.js";
import { type ISize } from "../../../base/common/layout.js";
import { DisposableOwner } from "../../../base/common/lifecycle.js";
import { type EditorSelectionController } from "../../common/cursor/editorSelectionController.js";
import { resolveEditorIndentationOptions, type EditorIndentationOptions, type ResolvedEditorIndentationOptions } from "../../common/editorIndentation.js";
import { type EditorLineVisibilitySource } from "../../common/viewModel/modelLineProjection.js";
import { TextPosition, type TextRange } from "../../common/core/text.js";
import { type TextModel } from "../../common/model/textModel.js";
import { type EditorVisualLineProjection } from "../../common/viewModel/modelLineProjection.js";
import { type EditorScrollPosition, type EditorViewportChange, type EditorViewportLayout, EditorViewportModel } from "../../common/viewLayout/editorViewportModel.js";
import { type DecorationSource } from "../viewparts/decorations/decorationPresentation.js";
import { DomTextMeasurer, type TextMeasurer } from "../measurement/fontMetrics.js";
import { LineWidthIndex } from "../measurement/lineWidthIndex.js";
import { type ClientPoint, type EditorHitTarget, EditorHitTargetKind, hitTestAsterVisualEditorPoint } from "../../common/viewModel/pointerHitTest.js";
import { getAsterDomTextCaretLeft, getAsterDomTextOffsetAtClientPoint } from "../viewparts/viewportOverlay/domTextGeometry.js";
import { type BracketColorizationSource, type SemanticTokenSource } from "../viewparts/semanticTokens/semanticTokenPresentation.js";
import { type ActiveLineHighlight, type ViewportOverlayContext } from "../viewparts/viewportOverlay/viewportOverlayPresentation.js";
import { EditorLineWrapping, VisualLineProjection } from "../viewModel/visualLineProjection.js";
import { VisibleLineProjection } from "../viewModel/visibleLineProjection.js";
import { getTextGraphemeBoundaries } from "../../common/core/textSegmentation.js";
import { type EditorLineGutterDecoration } from "../viewparts/margin/lineGutterDecoration.js";
import { BlockDecorationsPart } from "../viewparts/blockDecorations/blockDecorationsPart.js";
import { MarginPart } from "../viewparts/margin/marginPart.js";
import { MarginDecorationsPart } from "../viewparts/marginDecorations/marginDecorationsPart.js";
import { RulersPart, type EditorRuler } from "../viewparts/rulers/rulersPart.js";
import { EditorScrollbarPart } from "../viewparts/editorScrollbar/editorScrollbarPart.js";
import { CompositionPart } from "../viewparts/composition/compositionPart.js";
import { DecorationsPart } from "../viewparts/decorations/decorationsPart.js";
import { IndentGuidesPart } from "../viewparts/indentGuides/indentGuidesPart.js";
import { LineNumbersPart } from "../viewparts/lineNumbers/lineNumbersPart.js";
import { LinesDecorationsPart } from "../viewparts/linesDecorations/linesDecorationsPart.js";
import { MinimapPart } from "../viewparts/minimap/minimapPart.js";
import { OverviewRulerPart } from "../viewparts/overviewRuler/overviewRulerPart.js";
import { ScrollDecorationPart } from "../viewparts/scrollDecoration/scrollDecorationPart.js";
import { SelectionsPart } from "../viewparts/selections/selectionsPart.js";
import { ViewCursorsPart } from "../viewparts/viewCursors/viewCursorsPart.js";
import { EditorViewContext, EditorViewPartCollection } from "../viewparts/viewPart.js";
import { ViewLinesPart } from "../viewparts/viewLines/viewLinesPart.js";

export type EditorViewportPresentation = "document" | "embedded";

/** Chooses which component renders the visible focus outline for an Aster viewport. */
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

export type { EditorRuler } from "../viewparts/rulers/rulersPart.js";

/** Controls the browser paragraph direction used to shape Aster's rendered text. */
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
  readonly lineVisibilitySource?: EditorLineVisibilitySource;
  readonly lineGutterDecoration?: EditorLineGutterDecoration;
  readonly presentation?: EditorViewportPresentation;
  /** `host` delegates the visible focus outline to the viewport's direct host. */
  readonly focusOutlineOwner?: EditorFocusOutlineOwner;
  /** `off` omits current-line presentation while preserving selections and carets. */
  readonly activeLineHighlight?: EditorActiveLineHighlight;
  readonly lineWrapping?: EditorLineWrapping;
  readonly fontFamily?: string;
  readonly fontSize?: number;
  readonly fontLigatures?: boolean;
  readonly showLineNumbers?: boolean;
  readonly rulers?: readonly EditorRuler[];
  readonly showIndentationGuides?: boolean;
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
 * Read-only browser projection of one Aster text model.
 *
 * The common viewport owns layout math. This component owns the scroll host,
 * measurement inputs, and ordered visual-part lifecycle; individual parts own
 * their projected DOM and canvas surfaces.
 */
export class EditorViewport extends DisposableOwner {
  readonly element: HTMLDivElement;
  readonly onDidChangeLayout: Event<EditorViewportChange>;
  private readonly model: TextModel;
  private readonly viewport: EditorViewportModel;
  private readonly contentElement: HTMLDivElement;
  private readonly contentNode: FastDomNode<HTMLDivElement>;
  private readonly textMetricsElement: HTMLSpanElement;
  private readonly accessibilityStatusElement: HTMLDivElement;
  private readonly viewContext: EditorViewContext;
  private readonly viewParts: EditorViewPartCollection;
  private readonly viewLinesPart: ViewLinesPart;
  private readonly marginPart: MarginPart;
  private readonly decorationsPart: DecorationsPart;
  private readonly compositionPart: CompositionPart;
  private readonly selectionsPart: SelectionsPart;
  private readonly viewCursorsPart: ViewCursorsPart;
  private readonly textMeasurer: TextMeasurer;
  private readonly lineWidths: LineWidthIndex;
  private readonly visualLineProjection: VisualLineProjection;
  private readonly visibleLineProjection: VisibleLineProjection;
  private readonly lineGutterDecoration: EditorLineGutterDecoration | undefined;
  private readonly selectionController: EditorSelectionController | undefined;
  private readonly presentation: EditorViewportPresentation;
  private readonly focusOutlineOwner: EditorFocusOutlineOwner;
  private readonly activeLineHighlight: EditorActiveLineHighlight;
  private readonly showLineNumbers: boolean;
  private readonly showIndentationGuides: boolean;
  private readonly padding: EditorViewportPadding;
  private readonly indentation: ResolvedEditorIndentationOptions;
  private readonly minimap: EditorMinimap;
  private readonly textDirection: EditorTextDirection;
  private softWrapping: boolean;

  constructor(options: EditorViewportOptions) {
    super();
    const ownerDocument = options.container.ownerDocument;
    this.model = options.model;
    this.element = h(ownerDocument, "div");
    this.contentElement = h(ownerDocument, "div");
    this.contentNode = new FastDomNode(this.contentElement);
    this.textMetricsElement = h(ownerDocument, "span");
    this.accessibilityStatusElement = h(ownerDocument, "div");
    this.selectionController = options.selectionController;
    this.presentation = options.presentation ?? "document";
    this.focusOutlineOwner = options.focusOutlineOwner ?? "editor";
    this.activeLineHighlight = options.activeLineHighlight ?? (this.presentation === "embedded" ? "off" : "on");
    this.showLineNumbers = options.showLineNumbers ?? this.presentation !== "embedded";
    this.showIndentationGuides = options.showIndentationGuides ?? this.presentation !== "embedded";
    this.padding = resolveEditorViewportPadding(options.padding);
    this.minimap = options.minimap ?? (this.presentation === "document" ? EditorMinimap.On : EditorMinimap.Off);
    this.textDirection = options.textDirection ?? EditorTextDirection.Auto;
    this.softWrapping = options.lineWrapping === EditorLineWrapping.On;
    try {
      this.indentation = resolveEditorIndentationOptions(options.indentation);
      if (!Object.values(EditorMinimap).includes(this.minimap)) {
        throw new TypeError("Unknown Aster editor minimap mode");
      }
      if (!Object.values(EditorTextDirection).includes(this.textDirection)) {
        throw new TypeError("Unknown Aster editor text direction");
      }
      if (this.focusOutlineOwner !== "editor" && this.focusOutlineOwner !== "host") {
        throw new TypeError("Unknown Aster editor focus outline owner");
      }
      if (this.activeLineHighlight !== "on" && this.activeLineHighlight !== "off") {
        throw new TypeError("Unknown Aster editor active-line highlight");
      }
      if (this.selectionController && this.selectionController.textModel !== this.model) {
        throw new TypeError(
          "Aster viewport and selection controller must share one text model",
        );
      }
      if (options.semanticTokenSource && options.semanticTokenSource.textModel !== this.model) {
        throw new TypeError("Aster viewport and semantic token source must share one text model");
      }
      if (options.bracketColorizationSource && options.bracketColorizationSource.textModel !== this.model) {
        throw new TypeError("Aster viewport and bracket colorization source must share one text model");
      }
    } catch (error) {
      this.dispose();
      throw error;
    }
    this.lineGutterDecoration = options.lineGutterDecoration ? this.own(options.lineGutterDecoration) : undefined;
    this.element.className = "aster-editor";
    this.element.classList.add(`aster-editor-${this.presentation}`);
    this.element.classList.add(`aster-editor-focus-owner-${this.focusOutlineOwner}`);
    this.element.classList.add(`aster-editor-direction-${this.textDirection}`);
    this.element.classList.toggle("hide-line-numbers", !this.showLineNumbers);
    if (options.fontFamily) this.element.style.fontFamily = options.fontFamily;
    if (options.fontSize !== undefined) this.element.style.fontSize = `${options.fontSize}px`;
    this.element.style.fontVariantLigatures = options.fontLigatures ? "normal" : "none";
    this.element.style.tabSize = String(this.indentation.tabSize);
    this.element.style.setProperty("--aster-editor-padding-left", `${this.padding.left}px`);
    this.element.style.setProperty("--aster-editor-padding-right", `${this.padding.right}px`);
    this.element.dir = this.textDirection;
    this.element.classList.toggle("word-wrapped", this.softWrapping);
    this.element.tabIndex = 0;
    this.element.setAttribute("role", "region");
    this.element.setAttribute("aria-label", options.ariaLabel ?? "Aster editor");
    this.contentNode.setClassName("aster-editor-content");
    this.textMetricsElement.className =
      "aster-editor-text-metrics";
    this.textMetricsElement.setAttribute("aria-hidden", "true");
    this.accessibilityStatusElement.className = "aster-editor-accessibility-status";
    this.accessibilityStatusElement.setAttribute("aria-live", "polite");
    this.accessibilityStatusElement.setAttribute("aria-atomic", "true");
    this.element.append(this.contentElement, this.textMetricsElement, this.accessibilityStatusElement);
    options.container.append(this.element);
    this.defer(() => this.element.remove());
    this.textMeasurer =
      options.textMeasurer ??
      new DomTextMeasurer(this.textMetricsElement);
    this.lineWidths = this.own(new LineWidthIndex(
      this.model,
      this.textMeasurer,
      {
        initialMeasurement: {
          ...(this.model.largeFile.tooLargeForTokenization ? { maximumMeasuredLineCount: 2_048 } : {}),
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
      options.lineVisibilitySource,
    ));
    const viewport = this.own(new EditorViewportModel(this.model, {
      lineHeight: options.lineHeight,
      overscanLineCount: options.overscanLineCount,
      lineSource: this.visibleLineProjection.lineSource,
      padding: { top: this.padding.top, bottom: this.padding.bottom },
    }));
    this.viewport = viewport;
    this.onDidChangeLayout = viewport.onDidChange;
    this.viewContext = new EditorViewContext(
      () => viewport.layout,
      layout => this.overlayContext(layout),
    );
    this.viewParts = this.own(new EditorViewPartCollection());
    this.viewLinesPart = this.viewParts.register(new ViewLinesPart({
      ownerDocument,
      model: this.model,
      readVisualProjection: () => this.visualProjection,
      readProjectionRevision: () => this.visibleLineProjection.revision,
      semanticTokenSource: options.semanticTokenSource,
      bracketColorizationSource: options.bracketColorizationSource,
      lineGutterDecoration: this.lineGutterDecoration,
      textDirection: this.textDirection,
    }));
    this.marginPart = this.viewParts.register(new MarginPart({
      host: this.element,
      contentElement: this.contentElement,
      model: this.model,
      textMeasurer: this.textMeasurer,
      presentation: this.presentation,
      showLineNumbers: this.showLineNumbers,
      lineGutterDecoration: this.lineGutterDecoration,
      readVisualProjection: () => this.visualProjection,
      readRenderedLines: () => this.viewLinesPart.renderedLines,
    }));
    this.viewParts.register(new LineNumbersPart({
      showLineNumbers: this.showLineNumbers,
      readVisualProjection: () => this.visualProjection,
      readRenderedLines: () => this.viewLinesPart.renderedLines,
    }));
    this.decorationsPart = this.viewParts.register(new DecorationsPart(
      this.viewContext,
      this.model,
      options.decorationSources ?? [],
    ));
    this.viewParts.register(new LinesDecorationsPart(this.viewContext, this.decorationsPart));
    const blockDecorationsPart = this.viewParts.register(new BlockDecorationsPart(
      this.viewContext,
      this.decorationsPart,
      ownerDocument,
    ));
    this.viewParts.register(new MarginDecorationsPart(this.viewContext, this.decorationsPart));
    this.viewParts.register(new IndentGuidesPart(this.viewContext, {
      showIndentationGuides: this.showIndentationGuides,
      tabSize: this.indentation.tabSize,
    }));
    this.selectionsPart = this.viewParts.register(new SelectionsPart(this.viewContext, this.selectionController));
    this.viewCursorsPart = this.viewParts.register(new ViewCursorsPart(this.viewContext, this.selectionController));
    const rulersPart = this.viewParts.register(new RulersPart({
      ownerDocument,
      textMeasurer: this.textMeasurer,
      readTextLeft: () => this.textLeft,
      rulers: options.rulers,
    }));
    this.compositionPart = this.viewParts.register(new CompositionPart(this.viewContext, this.model));
    this.viewParts.register(new EditorScrollbarPart({
      container: this.element,
      viewport: this.element,
      scrollTo: position => this.scrollTo(position),
    }));
    const minimapPart = this.viewParts.register(new MinimapPart({
      ownerDocument,
      model: this.model,
      readLayout: () => this.viewport.layout,
      scrollTo: position => this.scrollTo(position),
      readMarkers: () => this.decorationsPart.overviewMarkers(),
      readMarkersRevision: () => this.decorationsPart.markersRevision,
      enabled: this.minimap === EditorMinimap.On,
    }));
    const overviewRulerPart = this.viewParts.register(new OverviewRulerPart({
      ownerDocument,
      minimapEnabled: this.minimap === EditorMinimap.On,
      readLineCount: () => this.model.lineCount,
      readMarkers: () => this.decorationsPart.overviewMarkers(),
      readMarkersRevision: () => this.decorationsPart.markersRevision,
    }));
    const scrollDecorationPart = this.viewParts.register(new ScrollDecorationPart(ownerDocument));

    // Root order is the visual stacking contract; Parts own nodes but do not choose their host.
    this.contentElement.append(
      this.viewLinesPart.domNode,
      this.marginPart.domNode,
      blockDecorationsPart.domNode,
      rulersPart.domNode,
    );
    this.element.append(
      minimapPart.domNode,
      overviewRulerPart.domNode,
      scrollDecorationPart.domNode,
    );
    this.own(this.visibleLineProjection.onDidChange(() => this.project(viewport.layout)));
    if (this.lineGutterDecoration) this.own(this.lineGutterDecoration.onDidChange(() => this.project(viewport.layout)));
    this.own(this.decorationsPart.onDidChange(() => this.project(viewport.layout)));
    viewport.setContentWidth(this.measuredContentWidth);

    this.own(viewport.onDidChange(({ layout }) => this.project(layout)));
    this.own(this.lineWidths.onDidChange(() => {
      viewport.setContentWidth(this.measuredContentWidth);
    }));
    this.own(addDisposableListener(this.element, "scroll", () => {
      const scrollPosition = {
        left: this.element.scrollLeft,
        top: this.element.scrollTop,
      };
      const layout = viewport.setScrollPosition(scrollPosition);
      this.syncScrollPosition(layout);
    }));
    this.own(this.model.onDidChange(change => {
      this.lineWidths.applyModelChange(change);
      if (this.softWrapping) this.updateWrapWidth(this.viewport.layout.viewportSize.width);
      viewport.setContentWidth(this.measuredContentWidth);
    }));
    if (this.selectionController) {
      this.own(this.selectionController.onDidChange(() => {
        this.selectionsPart.render(viewport.layout);
        this.viewCursorsPart.render(viewport.layout);
        this.updateAccessibilityStatus();
      }));
      this.updateAccessibilityStatus();
    }
    const semanticTokenSource = options.semanticTokenSource;
    if (semanticTokenSource) {
      this.own(semanticTokenSource.onDidChange(() => {
        this.viewLinesPart.renderVisibleLineText();
      }));
    }
    const fontSet = ownerDocument.fonts;
    if (fontSet) {
      this.own(addDisposableListener(fontSet, "loadingdone", () => {
        this.refreshFontMetrics();
      }));
    }

    this.own(observeResize(this.element, () => this.layout()));

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
      throw new TypeError("Unknown Aster editor line wrapping mode");
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
      throw new TypeError("Aster accessibility status must be a non-empty string");
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
    if (!Number.isFinite(horizontalOffset)) throw new RangeError("Aster visual cursor horizontal offset must be finite");
    if (this.textDirection === EditorTextDirection.LeftToRight) return undefined;
    const visualLine = this.visualProjection.lineAt(visualLineIndex);
    const line = this.viewLinesPart.renderedLines.get(visualLineIndex);
    if (!visualLine || !line) return undefined;
    const text = this.model.getLineContent(visualLine.logicalLineIndex).slice(visualLine.startColumn, visualLine.endColumn);
    if (line.textElement.textContent?.length !== text.length) return undefined;
    let nearestColumn: number | undefined;
    let nearestDistance = Number.POSITIVE_INFINITY;
    for (const column of getTextGraphemeBoundaries(text)) {
      const left = getAsterDomTextCaretLeft(line.textElement, column, line.domNode.domNode);
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
    this.compositionPart.setRange(range);
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
    return hitTestAsterVisualEditorPoint(
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
    for (const [visualLineIndex, renderedLine] of this.viewLinesPart.renderedLines) {
      const offset = getAsterDomTextOffsetAtClientPoint(
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
    const line = this.viewLinesPart.renderedLines.get(visualLineIndex);
    return line && Number.isSafeInteger(offset) && offset >= 0 && offset <= line.textElement.textContent?.length
      ? getAsterDomTextCaretLeft(line.textElement, offset, line.domNode.domNode)
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
    return this.marginPart.gutterWidth;
  }

  private get textLeft(): number {
    return this.marginPart.textLeft;
  }

  private updateWrapWidth(viewportWidth: number): void {
    this.visualLineProjection.setWrapWidth(Math.max(
      0,
      viewportWidth - this.gutterWidth - this.textMeasurer.horizontalPadding,
    ));
  }

  private project(layout: EditorViewportLayout): void {
    this.observeRenderedLineWidths(layout);
    if (layout !== this.viewport.layout) return;
    this.element.classList.toggle("horizontally-scrollable", layout.maximumScrollPosition.left > 0);
    this.element.classList.toggle("vertically-scrollable", layout.maximumScrollPosition.top > 0);
    this.contentNode.setWidth(layout.contentSize.width);
    this.contentNode.setHeight(layout.contentSize.height);
    this.viewParts.render(layout);
    this.syncScrollPosition(layout);
  }

  private observeRenderedLineWidths(layout: EditorViewportLayout): void {
    if (this.lineWidths.complete) return;
    const projection = this.visualProjection;
    const logicalLineIndexes = new Set<number>();
    for (let visualLineIndex = layout.renderLines.startLineIndex; visualLineIndex < layout.renderLines.endLineIndexExclusive; visualLineIndex += 1) {
      const visualLine = projection.lineAt(visualLineIndex);
      if (visualLine) logicalLineIndexes.add(visualLine.logicalLineIndex);
    }
    this.lineWidths.observeLines([...logicalLineIndexes]);
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

  private overlayContext(layout: EditorViewportLayout): ViewportOverlayContext {
    return {
      ownerDocument: this.element.ownerDocument,
      model: this.model,
      visualLineProjection: this.visualProjection,
      renderedLines: this.viewLinesPart.renderedLines,
      renderLines: layout.renderLines,
      textLeft: this.textLeft,
      textMeasurer: this.textMeasurer,
      useDomTextGeometry: this.textDirection !== EditorTextDirection.LeftToRight,
      activeLineHighlight: this.activeLineHighlight,
    };
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
      "Aster client point must contain finite coordinates",
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
    throw new RangeError(`Aster editor padding.${side} must be non-negative and finite`);
  }
  return value;
}

function clamp(value: number, minimum: number, maximum: number): number {
  return Math.min(Math.max(value, minimum), maximum);
}
