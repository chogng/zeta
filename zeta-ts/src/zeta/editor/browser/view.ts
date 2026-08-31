import { Event } from '../../base/common/event.js';
import { addDisposableListener, getClientArea, h, runWhenWindowIdle } from '../../base/browser/dom.js';
import { FastDomNode } from '../../base/browser/fastDomNode.js';
import { PixelRatio, type IPixelRatioMonitor } from '../../base/browser/pixelRatio.js';
import { type IDisposable, toDisposable } from '../../base/common/lifecycle.js';
import { type ISize } from '../../base/common/layout.js';
import { clamp, isFiniteNumber } from '../../base/common/numbers.js';
import { resolveEditorIndentationOptions, type EditorIndentationOptions, type ResolvedEditorIndentationOptions } from '../common/core/misc/indentation.js';
import { Position } from '../common/core/position.js';
import { type IDimension } from '../common/core/2d/dimension.js';
import { ScrollType } from '../common/editorCommon.js';
import { type Range } from '../common/core/range.js';
import { TextModel } from '../common/model/textModel.js';
import { TextDirection } from '../common/model.js';
import { type IViewModel } from '../common/viewModel.js';
import { ViewEventHandler } from '../common/viewEventHandler.js';
import * as viewEvents from '../common/viewEvents.js';
import { EditorVisualLineProjection } from '../common/viewModel/modelLineProjection.js';
import { type EditorScrollPosition } from '../common/viewModel/editorViewportContracts.js';
import { ComputeOptionsMemory, EditorLayoutInfoComputer, EditorLineWrapping, EditorOption, EditorOptions, type EditorLayoutInfo, type EditorMinimapLayoutInfo, type EditorMinimapOptions, type FindComputedEditorOptionValueById, type IEditorMinimapOptions, type IEditorOptions, type InternalEditorRenderLineNumbersOptions, type InternalGuidesOptions, RenderLineNumbersType, isWrappingIndent, TextEditorCursorStyle, WrappingIndent } from '../common/config/editorOptions.js';
import { type FontInfo } from '../common/config/fontInfo.js';
import { createBareFontInfoFromRawSettings } from '../common/config/fontInfoFromSettings.js';
import { type TextMeasurer } from '../common/viewModel/textMeasurer.js';
import { type EditorViewportChange, type EditorViewportLayout, ViewLayout } from '../common/viewLayout/viewLayout.js';
import { type ClientPoint, type EditorHitTarget, EditorHitTargetKind, hitTestStanzaVisualEditorPoint } from '../common/viewModel/pointerHitTest.js';
import { applyFontInfo } from './config/domFontInfo.js';
import { EditorConfiguration } from './config/editorConfiguration.js';
import { DecorationsOverlay, type DecorationSource } from './viewParts/decorations/decorations.js';
import { type BracketColorizationSource, type SemanticTokenSource } from './viewParts/viewLines/viewLine.js';
import { getTextGraphemeBoundaries } from '../common/core/textSegmentation.js';
import { Margin } from './viewParts/margin/margin.js';
import { GlyphMarginWidgets, resolveGlyphMarginLanes } from './viewParts/glyphMargin/glyphMargin.js';
import { Rulers, type EditorRuler } from './viewParts/rulers/rulers.js';
import { RulersGpu } from './viewParts/rulersGpu/rulersGpu.js';
import { EditorScrollbar } from './viewParts/editorScrollbar/editorScrollbar.js';
import { LineNumbersOverlay } from './viewParts/lineNumbers/lineNumbers.js';
import { BlockDecorations } from './viewParts/blockDecorations/blockDecorations.js';
import { CurrentLineHighlightOverlay } from './viewParts/currentLineHighlight/currentLineHighlight.js';
import { IndentGuidesOverlay } from './viewParts/indentGuides/indentGuides.js';
import { LinesDecorationsOverlay } from './viewParts/linesDecorations/linesDecorations.js';
import { MarginViewLineDecorationsOverlay } from './viewParts/marginDecorations/marginDecorations.js';
import { SelectionsOverlay } from './viewParts/selections/selections.js';
import { ViewCursors } from './viewParts/viewCursors/viewCursors.js';
import { WhitespaceOverlay } from './viewParts/whitespace/whitespace.js';
import { Minimap } from './viewParts/minimap/minimap.js';
import { DecorationsOverviewRuler } from './viewParts/overviewRuler/decorationsOverviewRuler.js';
import { ScrollDecorationViewPart } from './viewParts/scrollDecoration/scrollDecoration.js';
import { ViewContentWidgets } from './viewParts/contentWidgets/contentWidgets.js';
import { ViewOverlayWidgets } from './viewParts/overlayWidgets/overlayWidgets.js';
import { type ViewPart } from './view/viewPart.js';
import { ViewContext } from '../common/viewModel/viewContext.js';
import { type IColorTheme } from '../../platform/theme/common/colorTheme.js';
import type { IContentWidget, IOverlayWidget, IViewZoneChangeAccessor } from './editorBrowser.js';
import { ContentViewOverlays, MarginViewOverlays } from './view/viewOverlays.js';
import { LineWidthIndex, ViewLines } from './viewParts/viewLines/viewLines.js';
import { EditorTextDirection, ViewLineOptions } from './viewParts/viewLines/viewLineOptions.js';
import { ViewLinesGpu } from './viewParts/viewLinesGpu/viewLinesGpu.js';
import { ViewZones, type EditorViewZone, type EditorViewZoneHandle } from './viewParts/viewZones/viewZones.js';
import { linesDecorationsWidth } from './viewParts/linesDecorations/linesDecorations.js';
import { RenderingContext } from './view/renderingContext.js';
import { ViewportData } from '../common/viewLayout/viewLinesViewportData.js';
import './widget/codeEditor/editor.css';

const DEFAULT_EDITOR_SCROLLBAR = EditorOptions.scrollbar.defaultValue;
const EMPTY_LINE_INDEXES: ReadonlySet<number> = new Set();

export type EditorViewportPresentation = "document" | "embedded";

/** Chooses which component renders the visible focus outline for an Stanza viewport. */
export type EditorFocusOutlineOwner = "editor" | "host";

/** Space reserved around the editor's projected text rows. */
export interface EditorViewportPadding {
	readonly top: number;
	readonly right: number;
	readonly bottom: number;
	readonly left: number;
}

export type { EditorRuler } from "./viewParts/rulers/rulers.js";

/** Controls the browser paragraph direction used to shape Stanza's rendered text. */
export { EditorTextDirection };

export interface EditorViewportOptions {
	readonly container: HTMLElement;
	readonly viewModel: IViewModel;
	readonly configuration: EditorConfiguration;
	readonly theme: IColorTheme;
	readonly lineHeight?: number;
	readonly dimension?: IDimension;
	readonly automaticLayout?: boolean;
	readonly padding?: EditorViewportPadding;
	readonly ariaLabel?: string;
	readonly textMeasurer?: TextMeasurer & { refresh?(): boolean };
	readonly decorationSources?: readonly DecorationSource[];
	readonly semanticTokenSource?: SemanticTokenSource;
	readonly bracketColorizationSource?: BracketColorizationSource;
	readonly presentation?: EditorViewportPresentation;
	/** `host` delegates the visible focus outline to the viewport's direct host. */
	readonly focusOutlineOwner?: EditorFocusOutlineOwner;
	readonly renderLineHighlight?: IEditorOptions['renderLineHighlight'];
	readonly renderLineHighlightOnlyWhenFocus?: IEditorOptions['renderLineHighlightOnlyWhenFocus'];
	readonly lineWrapping?: EditorLineWrapping;
	readonly wrappingIndent?: WrappingIndent;
	readonly fontFamily?: string;
	readonly fontSize?: number;
	readonly fontLigatures?: boolean;
	readonly lineNumbers?: IEditorOptions['lineNumbers'];
	readonly glyphMargin?: boolean;
	readonly rulers?: readonly EditorRuler[];
	readonly guides?: IEditorOptions['guides'];
	readonly minimap?: IEditorMinimapOptions;
	readonly indentation?: EditorIndentationOptions;
	/** Browser text-direction input; automatic direction is the default. */
	readonly textDirection?: EditorTextDirection;
	readonly experimentalGpuAcceleration?: IEditorOptions['experimentalGpuAcceleration'];
	readonly renderWhitespace?: IEditorOptions['renderWhitespace'];
	readonly mouseStyle?: IEditorOptions['mouseStyle'];
	readonly cursorStyle?: IEditorOptions['cursorStyle'];
	readonly overtypeCursorStyle?: IEditorOptions['overtypeCursorStyle'];
	readonly cursorBlinking?: IEditorOptions['cursorBlinking'];
	readonly cursorSmoothCaretAnimation?: IEditorOptions['cursorSmoothCaretAnimation'];
	readonly cursorWidth?: IEditorOptions['cursorWidth'];
	readonly cursorHeight?: IEditorOptions['cursorHeight'];
	readonly allowOverflow?: IEditorOptions['allowOverflow'];
	readonly fixedOverflowWidgets?: IEditorOptions['fixedOverflowWidgets'];
	readonly cursorOptions?: IEditorOptions;
	readonly languageId?: string;
}

export interface EditorContentPosition {
	readonly left: number;
	readonly top: number;
	readonly height: number;
}

/** A caller-owned DOM root placed in vertical space between visual lines. */
export type { EditorViewZone, EditorViewZoneHandle } from './viewParts/viewZones/viewZones.js';

/**
 * Read-only browser projection of one Stanza text model.
 *
 * The common viewport owns layout math. This component owns the scroll host,
 * measurement inputs, and ordered visual-part lifecycle; individual parts own
 * their projected DOM and canvas surfaces.
 */
export class View extends ViewEventHandler {
	readonly element: HTMLDivElement;
	readonly onDidChangeLayout: Event<EditorViewportChange>;
	private _fontInfo: FontInfo;
	private readonly model: TextModel;
	private readonly viewport: ViewLayout;
	private readonly contentElement: HTMLDivElement;
	private readonly contentNode: FastDomNode<HTMLDivElement>;
	private readonly textMetricsElement: HTMLSpanElement;
	private readonly accessibilityStatusElement: HTMLDivElement;
	private readonly viewContext: ViewContext;
	private readonly viewParts: ViewPart[] = [];
	private readonly viewLines: ViewLines;
	private readonly viewLinesGpu: ViewLinesGpu | undefined;
	private readonly viewZones: ViewZones;
	private readonly contentWidgets: ViewContentWidgets;
	private readonly overlayWidgets: ViewOverlayWidgets;
	private readonly margin: Margin;
	private readonly contentViewOverlays: ContentViewOverlays;
	private readonly marginViewOverlays: MarginViewOverlays;
	private readonly decorations: DecorationsOverlay;
	private readonly viewCursors: ViewCursors;
	private readonly textMeasurer: TextMeasurer & { refresh?(): boolean };
	private readonly lineWidths: LineWidthIndex;
	private readonly viewModel: IViewModel;
	readonly coordinatesConverter: IViewModel['coordinatesConverter'];
	readonly cursorConfig: IViewModel['cursorConfig'];
	private readonly presentation: EditorViewportPresentation;
	private readonly focusOutlineOwner: EditorFocusOutlineOwner;
	private readonly renderLineHighlight: NonNullable<IEditorOptions['renderLineHighlight']>;
	private readonly renderLineHighlightOnlyWhenFocus: boolean;
	private readonly cursorStyle: TextEditorCursorStyle;
	private readonly overtypeCursorStyle: TextEditorCursorStyle;
	private readonly configuredCursorWidth: number;
	private readonly lineNumbers: InternalEditorRenderLineNumbersOptions;
	private readonly showGlyphMargin: boolean;
	private readonly guides: InternalGuidesOptions;
	private readonly padding: EditorViewportPadding;
	private readonly indentation: ResolvedEditorIndentationOptions;
	private readonly minimap: EditorMinimapOptions;
	private readonly minimapLayoutMemory = new ComputeOptionsMemory();
	private readonly viewLineOptions: ViewLineOptions;
	private readonly editorConfiguration: EditorConfiguration;
	private readonly pixelRatio: IPixelRatioMonitor;
	private changingLayout = false;
	private overlayWidgetsMinimumContentWidth = 0;
	private viewZonesMinimumContentWidth = 0;
	private softWrapping: boolean;
	private projectionRevision = 0;

	get currentLayout(): EditorViewportLayout {
		return this.viewport.layout;
	}

	getLayoutInfo(): EditorLayoutInfo {
		return this.editorConfiguration.options.get(EditorOption.layoutInfo);
	}

	getOption<T extends EditorOption>(id: T): FindComputedEditorOptionValueById<T> {
		return this.editorConfiguration.options.get(id);
	}

	get fontInfo(): FontInfo {
		return this._fontInfo;
	}

	constructor(options: EditorViewportOptions) {
		super();
		const ownerDocument = options.container.ownerDocument;
		const ownerWindow = ownerDocument.defaultView;
		if (!ownerWindow) throw new ReferenceError('Editor viewport requires a browser window');
		this.pixelRatio = PixelRatio.getInstance(ownerWindow);
		const bareFontInfo = createBareFontInfoFromRawSettings({
			fontFamily: options.fontFamily ?? options.cursorOptions?.fontFamily,
			fontWeight: options.cursorOptions?.fontWeight,
			fontSize: options.fontSize ?? options.cursorOptions?.fontSize,
			fontLigatures: options.fontLigatures ?? options.cursorOptions?.fontLigatures,
			fontVariations: options.cursorOptions?.fontVariations,
			lineHeight: options.lineHeight,
			letterSpacing: options.cursorOptions?.letterSpacing,
		}, this.pixelRatio.value, true);
		const lineHeight = bareFontInfo.lineHeight;
		if (!(options.viewModel.model instanceof TextModel)) throw new TypeError('Editor view requires the editor text model implementation');
		const viewport = options.viewModel.viewLayout;
		if (!(viewport instanceof ViewLayout)) throw new TypeError('Editor view requires the editor view layout implementation');
		this.viewModel = options.viewModel;
		this.model = options.viewModel.model;
		this.element = h(ownerDocument, "div");
		this.contentElement = h(ownerDocument, "div");
		this.contentNode = new FastDomNode(this.contentElement);
		this.textMetricsElement = h(ownerDocument, "span");
		this.accessibilityStatusElement = h(ownerDocument, "div");
		this.presentation = options.presentation ?? "document";
		this.focusOutlineOwner = options.focusOutlineOwner ?? "editor";
		this.renderLineHighlight = options.renderLineHighlight ?? (this.presentation === 'embedded' ? 'none' : 'line');
		this.renderLineHighlightOnlyWhenFocus = options.renderLineHighlightOnlyWhenFocus ?? false;
		const mouseStyle = EditorOptions.mouseStyle.validate(options.mouseStyle);
		this.cursorStyle = EditorOptions.cursorStyle.validate(options.cursorStyle);
		this.overtypeCursorStyle = EditorOptions.overtypeCursorStyle.validate(options.overtypeCursorStyle);
		const cursorBlinking = EditorOptions.cursorBlinking.validate(options.cursorBlinking);
		const cursorSmoothCaretAnimation = EditorOptions.cursorSmoothCaretAnimation.validate(
			options.cursorSmoothCaretAnimation,
		) as NonNullable<IEditorOptions['cursorSmoothCaretAnimation']>;
		this.configuredCursorWidth = EditorOptions.cursorWidth.validate(options.cursorWidth);
		const cursorHeight = EditorOptions.cursorHeight.validate(options.cursorHeight);
		this.lineNumbers = EditorOptions.lineNumbers.validate(options.lineNumbers ?? (this.presentation === 'embedded' ? 'off' : 'on'));
		this.showGlyphMargin = this.presentation !== 'embedded' && (options.glyphMargin ?? true);
		this.guides = EditorOptions.guides.validate({
			...options.guides,
			indentation: options.guides?.indentation ?? this.presentation !== 'embedded',
		});
		this.padding = resolveEditorViewportPadding(options.padding);
		this.minimap = EditorOptions.minimap.validate({
			...options.minimap,
			enabled: options.minimap?.enabled ?? this.presentation === 'document',
		}) as EditorMinimapOptions;
		this.softWrapping = options.lineWrapping === EditorLineWrapping.On;
		try {
			this.indentation = resolveEditorIndentationOptions(options.indentation);
			this.viewLineOptions = new ViewLineOptions({
				textDirection: options.textDirection ?? EditorTextDirection.Auto,
				fontLigatures: options.fontLigatures ?? false,
				useGpu: options.experimentalGpuAcceleration === 'on',
				useMonospaceOptimizations: false,
				lineHeight,
				tabSize: this.indentation.tabSize,
			});
			if (options.experimentalGpuAcceleration !== undefined && options.experimentalGpuAcceleration !== 'on' && options.experimentalGpuAcceleration !== 'off') {
				throw new TypeError("Unknown Stanza editor GPU acceleration mode");
			}
			if (this.focusOutlineOwner !== "editor" && this.focusOutlineOwner !== "host") {
				throw new TypeError("Unknown Stanza editor focus outline owner");
			}
			if (!['none', 'gutter', 'line', 'all'].includes(this.renderLineHighlight)) {
				throw new TypeError('Unknown Stanza editor line highlight mode');
			}
			if (typeof this.renderLineHighlightOnlyWhenFocus !== 'boolean') {
				throw new TypeError('Stanza editor line highlight focus option must be boolean');
			}
			if (options.semanticTokenSource && options.semanticTokenSource.textModel !== this.model) {
				throw new TypeError("Stanza viewport and semantic token source must share one text model");
			}
			if (options.bracketColorizationSource && options.bracketColorizationSource.textModel !== this.model) {
				throw new TypeError("Stanza viewport and bracket colorization source must share one text model");
			}
		} catch (error) {
			this.dispose();
			throw error;
		}
		this.element.className = "monaco-editor stanza-editor";
		this.element.classList.add(`stanza-editor-${this.presentation}`);
		this.element.classList.add(`stanza-editor-focus-owner-${this.focusOutlineOwner}`);
		this.element.classList.add(`stanza-editor-direction-${this.viewLineOptions.textDirection}`);
		this.element.classList.add(`stanza-editor-mouse-${mouseStyle}`);
		this.element.classList.toggle("hide-line-numbers", this.lineNumbers.renderType === RenderLineNumbersType.Off);
		applyFontInfo(this.element, bareFontInfo);
		this.element.style.tabSize = String(this.indentation.tabSize);
		this.element.style.setProperty("--stanza-editor-padding-left", `${this.padding.left}px`);
		this.element.style.setProperty("--stanza-editor-padding-right", `${this.padding.right}px`);
		this.element.dir = this.viewLineOptions.textDirection;
		this.element.classList.toggle("word-wrapped", this.softWrapping);
		this.element.tabIndex = 0;
		this.element.setAttribute("role", "region");
		this.element.setAttribute("aria-label", options.ariaLabel ?? "Stanza editor");
		this.contentNode.setClassName("stanza-editor-content");
		this.textMetricsElement.className =
			"stanza-editor-text-metrics";
		this.textMetricsElement.setAttribute("aria-hidden", "true");
		this.accessibilityStatusElement.className = "stanza-editor-accessibility-status";
		this.accessibilityStatusElement.setAttribute("aria-live", "polite");
		this.accessibilityStatusElement.setAttribute("aria-atomic", "true");
		this.element.append(this.contentElement, this.textMetricsElement, this.accessibilityStatusElement);
		options.container.append(this.element);
		this._register(toDisposable(() => this.element.remove()));
		this.textMeasurer =
			options.textMeasurer ??
			new BrowserTextMeasurer(this.textMetricsElement);
		this.editorConfiguration = options.configuration;
		this._fontInfo = this.editorConfiguration.options.get(EditorOption.fontInfo);
		applyFontInfo(this.element, this.fontInfo);
		const spaceWidth = this.fontInfo.spaceWidth;
		this.cursorConfig = this.viewModel.cursorConfig;
		const cursorWidth = Math.min(
			this.configuredCursorWidth,
			spaceWidth,
		);
		this.lineWidths = this._register(new LineWidthIndex(
			this.model,
			this.textMeasurer,
			{
				initialMeasurement: {
					...(this.model.largeFile.tooLargeForTokenization ? { maximumMeasuredLineCount: 2_048 } : {}),
					schedule: callback => runWhenWindowIdle(
						ownerWindow,
						() => callback(),
						250,
					),
				},
			},
		));
		this.coordinatesConverter = this.viewModel.coordinatesConverter;
		this.viewport = viewport;
		this.onDidChangeLayout = viewport.onDidChange;
		this.viewContext = new ViewContext(this.editorConfiguration, options.theme, this.viewModel);
		this.viewZones = this.registerViewPart(new ViewZones(this.viewContext, {
			host: this.element,
			viewLayout: this.viewport,
			readVisualLineCount: () => this.visualProjection.visualLineCount,
			readVisualLineIndexAfterPosition: (lineNumber, column) => {
				if (lineNumber === 0) return -1;
				const position = new Position(lineNumber, column ?? this.model.getLineMaxColumn(lineNumber));
				this.model.offsetAt(position);
				return this.visualProjection.visualLineIndexAt(position);
			},
			readContentLeft: () => this.contentOffsetLeft + this.textLeft,
			readContentWidth: () => Math.max(0, this.viewport.layout.viewportSize.width - this.contentOffsetLeft - this.textLeft),
			setMinimumContentWidth: width => this.setViewZonesMinimumContentWidth(width),
		}));
		this.contentWidgets = this.registerViewPart(new ViewContentWidgets(this.viewContext, {
			viewDomNode: this.element,
			allowOverflow: options.allowOverflow ?? true,
			fixedOverflowWidgets: options.fixedOverflowWidgets ?? false,
			readContentLeft: () => this.contentOffsetLeft,
			readContentWidth: () => Math.max(0, this.viewport.layout.viewportSize.width - this.contentOffsetLeft),
			model: this.model,
			readVisualProjection: () => this.visualProjection,
			readTextLeft: () => this.textLeft,
			textMeasurer: this.textMeasurer,
		}));
		this.overlayWidgets = this.registerViewPart(new ViewOverlayWidgets(this.viewContext, {
			viewDomNode: this.element,
			allowOverflow: options.allowOverflow ?? true,
			fixedOverflowWidgets: options.fixedOverflowWidgets ?? false,
			verticalScrollbarWidth: DEFAULT_EDITOR_SCROLLBAR.verticalScrollbarSize,
			horizontalScrollbarHeight: DEFAULT_EDITOR_SCROLLBAR.horizontalScrollbarSize,
			readMinimapWidth: () => this.computeMinimapLayout(this.viewport.layout.viewportSize.width, this.viewport.layout.viewportSize.height).minimapWidth,
			setMinimumContentWidth: width => this.setOverlayWidgetsMinimumContentWidth(width),
			requestRender: () => {
				if (!this.isDisposed) this.project(this.viewport.layout);
			},
		}));
		this.viewLines = this._register(new ViewLines({
			host: this.contentElement,
			model: this.model,
			readVisualProjection: () => this.visualProjection,
			readProjectionRevision: () => this.projectionRevision,
			semanticTokenSource: options.semanticTokenSource,
			bracketColorizationSource: options.bracketColorizationSource,
			viewLineOptions: this.viewLineOptions,
			typicalHalfwidthCharacterWidth: Math.max(1, this.textMeasurer.measureLineWidth(' ')),
			readGpuLineIndexes: () => this.viewLinesGpu?.gpuLineIndexes ?? EMPTY_LINE_INDEXES,
		}));
		this.viewLinesGpu = this.viewLineOptions.useGpu
			? this.registerViewPart(new ViewLinesGpu(this.viewContext, {
				host: this.element,
				viewLayout: this.viewport,
				model: this.model,
				readVisualProjection: () => this.visualProjection,
				readTextLeft: () => this.textLeft,
				semanticTokenSource: options.semanticTokenSource,
				bracketColorizationSource: options.bracketColorizationSource,
				paddingTop: this.padding.top,
				viewLineOptions: this.viewLineOptions,
				viewLines: this.viewLines,
				requestRender: () => {
					if (!this.isDisposed) this.project(this.viewport.layout);
				},
			}))
			: undefined;
		const decorationSources = Object.freeze([...(options.decorationSources ?? [])]);
		const glyphMarginSources = this.showGlyphMargin ? decorationSources : Object.freeze([]);
		const glyphMarginLanes = resolveGlyphMarginLanes(glyphMarginSources, this.showGlyphMargin);
		this.contentViewOverlays = this.registerViewPart(new ContentViewOverlays(this.viewContext, this.contentElement));
		this.decorations = new DecorationsOverlay(this.viewContext, this.model, decorationSources, this.element.ownerDocument, () => this.visualProjection, () => this.textLeft, this.textMeasurer);
		this.contentViewOverlays.addDynamicOverlay(new CurrentLineHighlightOverlay(this.viewContext, this.viewModel, this.element.ownerDocument, () => this.visualProjection, this.renderLineHighlight, this.renderLineHighlightOnlyWhenFocus));
		this.contentViewOverlays.addDynamicOverlay(new SelectionsOverlay(this.viewContext, this.viewModel, this.model, this.element.ownerDocument, () => this.visualProjection, () => this.textLeft, this.textMeasurer));
		this.contentViewOverlays.addDynamicOverlay(new IndentGuidesOverlay(this.viewContext, {
			guides: this.guides,
			tabSize: this.indentation.tabSize,
			bracketColorizationSource: options.bracketColorizationSource,
			viewModel: this.viewModel,
			ownerDocument: this.element.ownerDocument,
			readVisualProjection: () => this.visualProjection,
			readTextLeft: () => this.textLeft,
			textMeasurer: this.textMeasurer,
		}));
		this.contentViewOverlays.addDynamicOverlay(this.decorations);
		this.contentViewOverlays.addDynamicOverlay(new WhitespaceOverlay(
			this.viewContext,
			this.model,
			this.viewModel,
			options.renderWhitespace ?? 'none',
			this.element.ownerDocument,
			() => this.visualProjection,
			() => this.textLeft,
			this.textMeasurer,
		));
		this.viewCursors = this.registerViewPart(new ViewCursors(this.viewContext, {
			host: this.contentElement,
			style: this.cursorStyle,
			blinking: cursorBlinking,
			smoothCaretAnimation: cursorSmoothCaretAnimation,
			semanticTokenSource: options.semanticTokenSource,
			lineWidth: cursorWidth,
			lineHeight: cursorHeight,
			fontInfo: this.fontInfo,
			readVisualProjection: () => this.visualProjection,
			readTextLeft: () => this.textLeft,
			textMeasurer: this.textMeasurer,
			isRightToLeftAtPosition: position => this.viewModel.getTextDirection(position.lineNumber) === TextDirection.RTL,
		}, this.model, this.viewModel));
		const blockDecorations = this.registerViewPart(new BlockDecorations(this.viewContext, this.decorations, this.contentElement, () => this.visualProjection, () => this.textLeft));
		this.margin = this.registerViewPart(new Margin(this.viewContext, {
			host: this.element,
			contentElement: this.contentElement,
			model: this.model,
			textMeasurer: this.textMeasurer,
			presentation: this.presentation,
			showLineNumbers: this.lineNumbers.renderType !== RenderLineNumbersType.Off,
			glyphMarginLaneCount: glyphMarginLanes.length,
			lineHeight: this.fontInfo.lineHeight,
			lineDecorationsWidth: linesDecorationsWidth(decorationSources),
		}));
		this.marginViewOverlays = this.registerViewPart(new MarginViewOverlays(this.viewContext, this.contentElement));
		this.marginViewOverlays.addDynamicOverlay(new MarginViewLineDecorationsOverlay(this.viewContext, this.decorations, this.element.ownerDocument, () => this.visualProjection));
		this.marginViewOverlays.addDynamicOverlay(new LinesDecorationsOverlay(this.viewContext, this.decorations, decorationSources, this.element.ownerDocument, () => this.visualProjection));
		this.marginViewOverlays.addDynamicOverlay(new LineNumbersOverlay(this.viewContext, {
			lineNumbers: this.lineNumbers,
			viewModel: this.viewModel,
			readVisualProjection: () => this.visualProjection,
			ownerDocument: this.element.ownerDocument,
		}));
		const glyphMarginWidgets = this.registerViewPart(new GlyphMarginWidgets(this.viewContext, {
			host: this.contentElement,
			lanes: glyphMarginLanes,
			decorations: this.decorations,
			readVisualLines: () => this.visualProjection,
			readLeft: () => this.margin.glyphMarginLeft,
			readLaneWidth: () => this.margin.glyphMarginLaneWidth,
		}));
		this.margin.domNode.append(this.viewZones.marginDomNode, this.marginViewOverlays.getDomNode().domNode, glyphMarginWidgets.domNode);
		let rulersDomNode: HTMLElement | undefined;
		if (this.viewLinesGpu) {
			this.registerViewPart(new RulersGpu(
				this.viewContext,
				this.viewLinesGpu.gpuContext,
				Object.freeze([...(options.rulers ?? [])]),
				column => this.textLeft + this.textMeasurer.measureLineWidth('0'.repeat(column)),
			));
		} else {
			rulersDomNode = this.registerViewPart(new Rulers(this.viewContext, {
				host: this.contentElement,
				textMeasurer: this.textMeasurer,
				readTextLeft: () => this.textLeft,
				rulers: options.rulers,
			})).domNode;
		}
		this.registerViewPart(new EditorScrollbar(this.viewContext, {
			container: this.element,
			viewport: this.element,
			scrollTo: position => this.scrollTo(position),
			horizontalScrollbarSize: DEFAULT_EDITOR_SCROLLBAR.horizontalScrollbarSize,
			verticalScrollbarSize: DEFAULT_EDITOR_SCROLLBAR.verticalScrollbarSize,
		}));
		const minimapPart = this.registerViewPart(new Minimap(this.viewContext, {
			host: this.element,
			model: this.model,
			options: this.minimap,
			semanticTokenSource: options.semanticTokenSource,
			tabSize: this.indentation.tabSize,
			paddingTop: this.padding.top,
			paddingBottom: this.padding.bottom,
			readLayout: () => this.viewport.layout,
			readMinimapLayout: () => this.computeMinimapLayout(this.viewport.layout.viewportSize.width, this.viewport.layout.viewportSize.height),
			readVisualProjection: () => this.visualProjection,
			readProjectionRevision: () => this.projectionRevision,
			scrollTo: position => this.scrollTo(position),
			readMarkers: () => this.decorations.minimapMarkers(),
			readMarkersRevision: () => this.decorations.markersRevision,
		}));
		const decorationsOverviewRuler = this.registerViewPart(new DecorationsOverviewRuler(this.viewContext, {
			host: this.element,
			verticalScrollbarWidth: DEFAULT_EDITOR_SCROLLBAR.verticalScrollbarSize,
			getVerticalOffsetForLineIndex: lineIndex => this.viewport.getVerticalOffsetForLineIndex(
				lineIndex >= this.model.lineCount
					? this.visualProjection.visualLineCount
					: this.visualProjection.visualLineIndexAt(new Position((lineIndex) + 1, (0) + 1)),
			),
			readMarkers: () => this.decorations.overviewMarkers(),
			readMarkersRevision: () => this.decorations.markersRevision,
		}));
		const scrollDecoration = this.registerViewPart(new ScrollDecorationViewPart(this.viewContext, this.element));

		// Root order is the visual stacking contract; Parts own nodes but do not choose their host.
		this.contentElement.append(
			this.viewLines.domNode,
			this.contentViewOverlays.getDomNode().domNode,
			this.viewCursors.domNode,
			this.contentWidgets.domNode.domNode,
			this.margin.domNode,
			blockDecorations.domNode,
			...(rulersDomNode ? [rulersDomNode] : []),
		);
		this.element.append(
			this.overlayWidgets.getDomNode().domNode,
			minimapPart.domNode,
			decorationsOverviewRuler.domNode,
			scrollDecoration.domNode,
			this.viewZones.domNode,
		);
		ownerDocument.body.append(
			this.contentWidgets.overflowingContentWidgetsDomNode.domNode,
			this.overlayWidgets.overflowingOverlayWidgetsDomNode.domNode,
		);
		this._register(this.decorations.onDidChange(() => this.project(viewport.layout)));
		viewport.setMaxLineWidth(this.measuredContentWidth);

		this._register(this.lineWidths.onDidChange(() => {
			viewport.setMaxLineWidth(this.measuredContentWidth);
			queueMicrotask(() => {
				if (!this.isDisposed) this.project(viewport.layout);
			});
		}));
		this._register(addDisposableListener(this.element, "scroll", () => {
			viewport.setScrollPosition({
				scrollLeft: this.element.scrollLeft,
				scrollTop: this.element.scrollTop,
			}, ScrollType.Immediate);
			this.syncScrollPosition(viewport.layout);
		}));
		this._register(this.model.onDidChangeContent(change => {
			this.lineWidths.applyModelChange(change);
			viewport.setMaxLineWidth(this.measuredContentWidth);
		}));
		this.updateAccessibilityStatus();
		const semanticTokenSource = options.semanticTokenSource;
		if (semanticTokenSource) {
			this._register(semanticTokenSource.onDidChange(() => {
				this.viewLines.renderVisibleLineText();
				this.viewLinesGpu?.invalidateTokens();
				const context = this.createRenderingContext(this.createViewportData());
				this.viewLinesGpu?.render(context);
				this.viewCursors.renderTokens(context);
				minimapPart.renderNow(context);
			}));
		}
		const fontSet = ownerDocument.fonts;
		if (fontSet) {
			this._register(addDisposableListener(fontSet, "loadingdone", () => {
				this.refreshFontMetrics();
			}));
		}

		this._register(this.editorConfiguration.onDidChange(event => {
			if (event.hasChanged(EditorOption.fontInfo)) this.applyFontConfiguration();
			if (event.hasChanged(EditorOption.wrappingInfo)) {
				this.softWrapping = this.editorConfiguration.options.get(EditorOption.wrappingInfo).wrappingColumn > 0;
				this.element.classList.toggle("word-wrapped", this.softWrapping);
			}
			if (!this.changingLayout && event.hasChanged(EditorOption.layoutInfo)) this.project(viewport.layout);
		}));
		this._register(this.pixelRatio.onDidChange(() => this.project(viewport.layout)));
		this.viewModel.addViewEventHandler(this);
		this._register(toDisposable(() => this.viewModel.removeViewEventHandler(this)));
		this.layout(options.dimension ?? getClientArea(this.element));
		this.onDidRender();
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

	get wrappingIndent(): WrappingIndent {
		return this.editorConfiguration.options.get(EditorOption.wrappingIndent);
	}

	/** Returns the browser paragraph direction used by the text projection. */
	get editorTextDirection(): EditorTextDirection {
		return this.viewLineOptions.textDirection;
	}

	/** Changes only this viewport's visual row projection; document text is unaffected. */
	setLineWrapping(lineWrapping: EditorLineWrapping): EditorViewportLayout {
		if (!Object.values(EditorLineWrapping).includes(lineWrapping)) {
			throw new TypeError("Unknown Stanza editor line wrapping mode");
		}
		const nextSoftWrapping = lineWrapping === EditorLineWrapping.On;
		if (nextSoftWrapping === this.softWrapping) return this.viewport.layout;
		this.softWrapping = nextSoftWrapping;
		this.editorConfiguration.updateOptions({ wordWrap: nextSoftWrapping ? 'on' : 'off' });
		this.element.classList.toggle("word-wrapped", nextSoftWrapping);
		this.viewport.setMaxLineWidth(this.measuredContentWidth);
		const layout = this.viewport.layout;
		this.project(layout);
		return layout;
	}

	setWrappingIndent(wrappingIndent: WrappingIndent): EditorViewportLayout {
		if (!isWrappingIndent(wrappingIndent)) {
			throw new TypeError("Unknown Stanza wrapping indent mode");
		}
		if (wrappingIndent === this.wrappingIndent) return this.viewport.layout;
		this.editorConfiguration.updateOptions({ wrappingIndent: rawWrappingIndent(wrappingIndent) });
		this.viewport.setMaxLineWidth(this.measuredContentWidth);
		const layout = this.viewport.layout;
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
		this.changingLayout = true;
		try {
			this.editorConfiguration.observeContainer(size);
			return this.layoutViewport(size);
		} finally {
			this.changingLayout = false;
		}
	}

	private layoutViewport(size: ISize): EditorViewportLayout {
		void size;
		this.refreshFontMetrics();
		const layout = this.viewport.layout;
		this.project(layout);
		return layout;
	}

	/** Announces one editor status message through the viewport's live region. */
	announceAccessibilityStatus(message: string): void {
		if (typeof message !== "string" || message.trim().length === 0) {
			throw new TypeError("Stanza accessibility status must be a non-empty string");
		}
		this.accessibilityStatusElement.textContent = message.trim();
	}

	refreshFontMetrics(force = false): EditorViewportLayout {
		if (!this.textMeasurer.refresh?.() && !force) return this.viewport.layout;
		this.viewLinesGpu?.invalidateFont();
		this.viewCursors.setLineWidth(Math.min(
			this.configuredCursorWidth,
			Math.max(1, this.textMeasurer.measureLineWidth(' ')),
		));
		this.lineWidths.refresh();
		this.viewport.setMaxLineWidth(this.measuredContentWidth);
		const layout = this.viewport.layout;
		this.project(layout);
		return layout;
	}

	private applyFontConfiguration(): void {
		this._fontInfo = this.editorConfiguration.options.get(EditorOption.fontInfo);
		applyFontInfo(this.element, this._fontInfo);
		this.margin.setLineHeight(this._fontInfo.lineHeight);
		this.viewZones.setLineHeight(this._fontInfo.lineHeight);
		this.refreshFontMetrics(true);
	}

	setLineHeight(lineHeight: number): EditorViewportLayout {
		this.editorConfiguration.updateOptions({ lineHeight });
		return this.viewport.layout;
	}

	/** Mounts one caller-owned view zone and returns its layout lifetime. */
	addViewZone(zone: EditorViewZone): EditorViewZoneHandle {
		return this.viewZones.addZone(zone);
	}

	changeViewZones(callback: (accessor: IViewZoneChangeAccessor) => void): void {
		this.viewZones.changeViewZones(callback);
	}

	addContentWidget(widget: IContentWidget): void {
		this.contentWidgets.addWidget(widget);
		this.layoutContentWidget(widget);
	}

	layoutContentWidget(widget: IContentWidget): void {
		this.contentWidgets.setWidgetPosition(widget, widget.getPosition());
		this.project(this.viewport.layout);
	}

	removeContentWidget(widget: IContentWidget): void {
		this.contentWidgets.removeWidget(widget);
	}

	addOverlayWidget(widget: IOverlayWidget): void {
		this.overlayWidgets.addWidget(widget);
		this.layoutOverlayWidget(widget);
	}

	layoutOverlayWidget(widget: IOverlayWidget): void {
		if (this.overlayWidgets.setWidgetPosition(widget, widget.getPosition())) {
			this.project(this.viewport.layout);
		}
	}

	removeOverlayWidget(widget: IOverlayWidget): void {
		this.overlayWidgets.removeWidget(widget);
		this.project(this.viewport.layout);
	}

	scrollTo(position: EditorScrollPosition): EditorViewportLayout {
		this.viewport.setScrollPosition({ scrollLeft: position.left, scrollTop: position.top }, ScrollType.Immediate);
		const layout = this.viewport.layout;
		this.project(layout);
		return layout;
	}

	revealPosition(position: Position): EditorViewportLayout {
		this.model.offsetAt(position);
		const layout = this.viewport.layout;
		const visualProjection = this.visualProjection;
		const visualLineIndex = visualProjection.visualLineIndexAt(position);
		const visualLine = visualProjection.lineAt(visualLineIndex)!;
		const lineTop = this.viewport.getVerticalOffsetForLineIndex(visualLineIndex);
		const lineBottom = lineTop + layout.lineHeight;
		let top = layout.scrollPosition.top;
		if (lineTop < top) {
			top = lineTop;
		} else if (lineBottom > top + layout.viewportSize.height) {
			top = lineBottom - layout.viewportSize.height;
		}

		const line = this.model.getLineContent((visualLine.logicalLineIndex) + 1);
		const columnIndex = position.column - 1;
		const domCaretLeft = this.domCaretLeft(visualLineIndex, columnIndex - visualLine.startColumn);
		const caretLeft = domCaretLeft ?? (this.contentTextLeft + (visualLine.wrappedTextIndentWidth ?? 0) +
			this.textMeasurer.measureLineWidth(line.slice(visualLine.startColumn, columnIndex)));
		const caretRight = caretLeft + Math.max(
			1,
			this.textMeasurer.measureLineWidth(" "),
		);
		let left = this.softWrapping ? 0 : layout.scrollPosition.left;
		if (caretLeft < left + this.contentTextLeft) {
			left = caretLeft - this.contentTextLeft;
		} else if (caretRight > left + layout.viewportSize.width) {
			left = caretRight - layout.viewportSize.width;
		}
		return this.scrollTo({ left, top });
	}

	getPositionContentCoordinates(position: Position): EditorContentPosition {
		this.model.offsetAt(position);
		const visualProjection = this.visualProjection;
		const visualLineIndex = visualProjection.visualLineIndexAt(position);
		const visualLine = visualProjection.lineAt(visualLineIndex)!;
		const columnIndex = position.column - 1;
		const domCaretLeft = this.domCaretLeft(visualLineIndex, columnIndex - visualLine.startColumn);
		return Object.freeze({
			left: domCaretLeft ?? (this.contentTextLeft + (visualLine.wrappedTextIndentWidth ?? 0) + this.textMeasurer.measureLineWidth(
				this.model.getLineContent((visualLine.logicalLineIndex) + 1).slice(visualLine.startColumn, columnIndex),
			)),
			top: this.viewport.getVerticalOffsetForLineIndex(visualLineIndex),
			height: this.viewport.layout.lineHeight,
		});
	}

	/** Returns a browser-shaped x-coordinate for one rendered visual cursor, when available. */
	getVisualHorizontalOffset(position: Position): number | undefined {
		this.model.offsetAt(position);
		const visualLineIndex = this.visualProjection.visualLineIndexAt(position);
		const visualLine = this.visualProjection.lineAt(visualLineIndex);
		return visualLine
			? this.domCaretLeft(visualLineIndex, position.column - 1 - visualLine.startColumn)
			: undefined;
	}

	/** Resolves the nearest browser-shaped cursor on one currently rendered visual line. */
	getNearestPositionAtVisualHorizontalOffset(visualLineIndex: number, horizontalOffset: number): Position | undefined {
		if (!isFiniteNumber(horizontalOffset)) throw new RangeError("Stanza visual cursor horizontal offset must be finite");
		if (this.viewLineOptions.textDirection === EditorTextDirection.LeftToRight) return undefined;
		const visualLine = this.visualProjection.lineAt(visualLineIndex);
		const line = this.viewLines.renderedLines.get(visualLineIndex);
		if (!visualLine || !line) return undefined;
		const text = this.model.getLineContent((visualLine.logicalLineIndex) + 1).slice(visualLine.startColumn, visualLine.endColumn);
		if (line.textElement.textContent?.length !== text.length) return undefined;
		let nearestColumn: number | undefined;
		let nearestDistance = Number.POSITIVE_INFINITY;
		for (const column of getTextGraphemeBoundaries(text)) {
			const left = line.getCaretLeft(column);
			if (left === undefined) return undefined;
			const distance = Math.abs(left - horizontalOffset);
			if (distance < nearestDistance) {
				nearestColumn = column;
				nearestDistance = distance;
			}
		}
		return nearestColumn === undefined
			? undefined
			: new Position((visualLine.logicalLineIndex) + 1, (visualLine.startColumn + nearestColumn) + 1);
	}

	setCompositionRange(range: Range | undefined): void {
		this.viewCursors.setCompositionRange(range);
		this.project(this.viewport.layout);
	}

	setOvertype(overtyping: boolean): void {
		this.viewCursors.setStyle(overtyping ? this.overtypeCursorStyle : this.cursorStyle);
		this.project(this.viewport.layout);
	}

	getTargetAtClientPoint(
		point: ClientPoint,
	): EditorHitTarget | undefined {
		validateClientPoint(point);
		const domTarget = this.viewLineOptions.textDirection === EditorTextDirection.LeftToRight
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
		const domTarget = this.viewLineOptions.textDirection === EditorTextDirection.LeftToRight
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
		return hitTestStanzaVisualEditorPoint(
			this.model,
			this.visualProjection,
			this.viewport.layout,
			{ left: left - this.contentOffsetLeft, top },
			{
				gutterWidth: this.gutterWidth,
				textLeft: this.textLeft,
				paddingTop: this.padding.top,
				getLineIndexAtVerticalOffset: offset => this.viewport.getLineIndexAtVerticalOffset(offset),
			},
			this.textMeasurer,
		);
	}

	private getDomTargetAtClientPoint(point: ClientPoint): EditorHitTarget | undefined {
		for (const [visualLineIndex, renderedLine] of this.viewLines.renderedLines) {
			const offset = renderedLine.getOffsetAtClientPoint(point.clientX, point.clientY);
			if (offset === undefined) continue;
			const visualLine = this.visualProjection.lineAt(visualLineIndex);
			if (!visualLine) continue;
			return Object.freeze({
				kind: EditorHitTargetKind.Text,
				position: new Position((visualLine.logicalLineIndex) + 1, (visualLine.startColumn + offset) + 1),
			});
		}
		return undefined;
	}

	private domCaretLeft(visualLineIndex: number, offset: number): number | undefined {
		if (this.viewLineOptions.textDirection === EditorTextDirection.LeftToRight) return undefined;
		const line = this.viewLines.renderedLines.get(visualLineIndex);
		return line?.hasTextOffset(offset)
			? line.getCaretLeft(offset)
			: undefined;
	}

	private get measuredContentWidth(): number {
		const textContentWidth = this.softWrapping ? 0 : Math.ceil(
			this.gutterWidth +
			this.lineWidths.maximumLineWidth +
			this.textMeasurer.horizontalPadding,
		);
		return Math.max(textContentWidth, this.overlayWidgetsMinimumContentWidth, this.viewZonesMinimumContentWidth);
	}

	private setOverlayWidgetsMinimumContentWidth(width: number): void {
		if (width === this.overlayWidgetsMinimumContentWidth) return;
		this.overlayWidgetsMinimumContentWidth = width;
		this.viewport.setMaxLineWidth(this.measuredContentWidth);
	}

	private setViewZonesMinimumContentWidth(width: number): void {
		if (width === this.viewZonesMinimumContentWidth) return;
		this.viewZonesMinimumContentWidth = width;
		this.viewport.setMaxLineWidth(this.measuredContentWidth);
	}

	private get gutterWidth(): number {
		return this.margin.gutterWidth;
	}

	private get textLeft(): number {
		return this.margin.textLeft;
	}

	private get contentTextLeft(): number {
		return this.contentOffsetLeft + this.textLeft;
	}

	private get contentOffsetLeft(): number {
		const layout = this.viewport.layout;
		const minimapLayout = this.computeMinimapLayout(layout.viewportSize.width, layout.viewportSize.height);
		return this.minimap.side === 'left' ? minimapLayout.minimapWidth : 0;
	}

	private computeMinimapLayout(viewportWidth: number, viewportHeight: number): EditorMinimapLayoutInfo {
		return EditorLayoutInfoComputer._computeMinimapLayout({
			outerWidth: viewportWidth,
			outerHeight: viewportHeight,
			lineHeight: this.viewport.layout.lineHeight,
			typicalHalfwidthCharacterWidth: Math.max(1, this.textMeasurer.measureLineWidth('n')),
			pixelRatio: this.pixelRatio.value,
			scrollBeyondLastLine: false,
			paddingTop: this.padding.top,
			paddingBottom: this.padding.bottom,
			minimap: this.minimap,
			verticalScrollbarWidth: DEFAULT_EDITOR_SCROLLBAR.verticalScrollbarSize,
			viewLineCount: this.visualProjection.visualLineCount,
			remainingWidth: Math.max(0, viewportWidth - this.gutterWidth),
			isViewportWrapping: this.softWrapping,
		}, this.minimapLayoutMemory);
	}

	private project(layout: EditorViewportLayout): void {
		this.observeRenderedLineWidths(layout);
		if (layout !== this.viewport.layout) return;
		const startLineNumber = layout.visibleLines.startLineIndex + 1;
		const endLineNumber = layout.visibleLines.endLineIndexExclusive;
		if (startLineNumber <= endLineNumber) {
			this.viewModel.setViewport(startLineNumber, endLineNumber, Math.floor((startLineNumber + endLineNumber) / 2));
			this.viewModel.visibleLinesStabilized();
		}
		const viewportData = this.createViewportData();
		this.viewLines.render(viewportData);
		const context = this.createRenderingContext(viewportData);
		this.element.classList.toggle("horizontally-scrollable", layout.maximumScrollPosition.left > 0);
		this.element.classList.toggle("vertically-scrollable", layout.maximumScrollPosition.top > 0);
		this.contentNode.setWidth(layout.contentSize.width);
		this.contentNode.setHeight(layout.contentSize.height);
		const contentOffsetLeft = this.contentOffsetLeft;
		this.contentNode.setTransform(contentOffsetLeft > 0 ? `translate3d(${contentOffsetLeft}px, 0, 0)` : '');
		for (const part of this.viewParts) part.onBeforeRender(context.viewportData);
		for (const part of this.viewParts) part.prepareRender(context);
		for (const part of this.viewParts) {
			part.render(context);
			part.onDidRender();
		}
		this.syncScrollPosition(layout);
		this.onDidRender();
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
		const selectionSet = this.viewModel.getCursorStates().map(state => state.modelState.selection);
		const selection = selectionSet[0]!;
		if (!selection) return;
		const position = selection.getPosition();
		const selectedLength = this.model.offsetAt(selection.getEndPosition()) - this.model.offsetAt(selection.getStartPosition());
		if (selectionSet.length > 1) {
			const totalSelectedLength = selectionSet.reduce((length, current) =>
				length + this.model.offsetAt(current.getEndPosition()) - this.model.offsetAt(current.getStartPosition()), 0);
			const summary = totalSelectedLength === 0
				? `${selectionSet.length} cursors`
				: `${selectionSet.length} selections, ${totalSelectedLength} characters selected`;
			this.announceAccessibilityStatus(`${summary}; primary at Line ${position.lineNumber}, column ${position.column}`);
			return;
		}
		this.announceAccessibilityStatus(selectedLength === 0
			? `Line ${position.lineNumber}, column ${position.column}`
			: `Line ${position.lineNumber}, column ${position.column}, ${selectedLength} characters selected`);
	}

	private createViewportData(): ViewportData {
		return new ViewportData(
			this.viewModel.getCursorStates().map(state => state.viewState.selection),
			this.viewport.getLinesViewportData(),
			this.viewport.getWhitespaceViewportData(),
			this.viewModel,
		);
	}

	private createRenderingContext(viewportData: ViewportData): RenderingContext {
		return new RenderingContext(this.viewport, viewportData, this.viewLines, this.viewLinesGpu);
	}

	private get visualProjection(): EditorVisualLineProjection {
		return createVisualProjection(this.model, this.viewModel, this.fontInfo.spaceWidth);
	}

	private registerViewPart<T extends ViewPart>(part: T): T {
		this.viewParts.push(part);
		return this._register(part);
	}

	private syncScrollPosition(layout: EditorViewportLayout): void {
		if (this.element.scrollLeft !== layout.scrollPosition.left) {
			this.element.scrollLeft = layout.scrollPosition.left;
		}
		if (this.element.scrollTop !== layout.scrollPosition.top) {
			this.element.scrollTop = layout.scrollPosition.top;
		}
	}

	public override handleEvents(events: viewEvents.ViewEvent[]): void {
		super.handleEvents(events);
		if (this.changingLayout || !this.shouldRender() || this.isDisposed) return;
		this.project(this.viewport.layout);
	}

	public override onConfigurationChanged(): boolean {
		this.projectionRevision += 1;
		return true;
	}

	public override onCursorStateChanged(): boolean {
		this.updateAccessibilityStatus();
		return true;
	}

	public override onFlushed(): boolean {
		this.projectionRevision += 1;
		return true;
	}

	public override onLineMappingChanged(): boolean {
		this.projectionRevision += 1;
		return true;
	}
}

function createVisualProjection(model: TextModel, viewModel: IViewModel, spaceWidth: number): EditorVisualLineProjection {
	const lines = Array.from({ length: viewModel.getLineCount() }, (_, index) => {
		const lineNumber = index + 1;
		const data = viewModel.getViewLineData(lineNumber);
		const start = viewModel.coordinatesConverter.convertViewPositionToModelPosition(new Position(lineNumber, data.minColumn));
		const end = viewModel.coordinatesConverter.convertViewPositionToModelPosition(new Position(lineNumber, data.maxColumn));
		return {
			visualLineIndex: index,
			logicalLineIndex: start.lineNumber - 1,
			startColumn: start.column - 1,
			endColumn: end.column - 1,
			firstForLogicalLine: index === 0 || viewModel.coordinatesConverter.convertViewPositionToModelPosition(new Position(index, viewModel.getLineMaxColumn(index))).lineNumber !== start.lineNumber,
			lastForLogicalLine: !data.continuesWithWrappedLine,
			...(data.startVisibleColumn > 0 ? { wrappedTextIndentWidth: data.startVisibleColumn * spaceWidth } : {}),
		};
	});
	const anchors = Array.from({ length: model.lineCount }, (_, index) => viewModel.coordinatesConverter.getViewLineNumberOfModelPosition(index + 1, 1) - 1);
	return EditorVisualLineProjection.fromVisibleLines(model.version, model.lineCount, lines, anchors);
}

function rawWrappingIndent(value: WrappingIndent): 'none' | 'same' | 'indent' | 'deepIndent' {
	switch (value) {
		case WrappingIndent.None: return 'none';
		case WrappingIndent.Same: return 'same';
		case WrappingIndent.Indent: return 'indent';
		case WrappingIndent.DeepIndent: return 'deepIndent';
	}
}

interface BrowserTextMetrics {
	readonly signature: string;
	readonly font: string;
	readonly fontSize: number;
	readonly letterSpacing: number;
	readonly spaceWidth: number;
	readonly tabSize: number;
	readonly horizontalPadding: number;
	readonly contentLeftPadding: number;
}

class BrowserTextMeasurer implements TextMeasurer {
	private readonly context: CanvasRenderingContext2D | undefined;
	private metrics: BrowserTextMetrics;

	constructor(private readonly referenceElement: HTMLElement) {
		try {
			this.context = h(referenceElement.ownerDocument, 'canvas').getContext('2d') ?? undefined;
		} catch {
			this.context = undefined;
		}
		this.metrics = this.readMetrics();
	}

	get horizontalPadding(): number { return this.metrics.horizontalPadding; }
	get contentLeftPadding(): number { return this.metrics.contentLeftPadding; }

	refresh(): boolean {
		const metrics = this.readMetrics();
		if (metrics.signature === this.metrics.signature) return false;
		this.metrics = metrics;
		return true;
	}

	measureLineWidth(text: string): number {
		const tabStopWidth = Math.max(1, this.metrics.spaceWidth * this.metrics.tabSize);
		let width = 0;
		const segments = text.split('\t');
		for (let index = 0; index < segments.length; index += 1) {
			width += this.measureSegment(segments[index]!);
			if (index + 1 < segments.length) width = (Math.floor(width / tabStopWidth) + 1) * tabStopWidth;
		}
		return width;
	}

	private readMetrics(): BrowserTextMetrics {
		const ownerWindow = this.referenceElement.ownerDocument.defaultView;
		if (!ownerWindow) throw new ReferenceError('Editor text measurement requires a browser window');
		const style = ownerWindow.getComputedStyle(this.referenceElement);
		const fontSize = cssNumber(style.fontSize, 14);
		const letterSpacing = style.letterSpacing === 'normal' ? 0 : cssNumber(style.letterSpacing, 0);
		const tabSize = Math.max(1, cssNumber(style.tabSize, 4));
		const contentLeftPadding = cssNumber(style.paddingLeft, 0);
		const horizontalPadding = contentLeftPadding + cssNumber(style.paddingRight, 0);
		const font = `${style.fontStyle || 'normal'} ${style.fontVariantCaps || 'normal'} ${style.fontWeight || '400'} ${style.fontSize || `${fontSize}px`} ${style.fontFamily || 'monospace'}`;
		if (this.context) {
			this.context.font = font;
			this.context.textBaseline = 'alphabetic';
		}
		const spaceWidth = positiveNumber(this.context?.measureText(' ').width, fontSize * 0.6);
		return Object.freeze({
			signature: JSON.stringify([font, letterSpacing, style.fontFeatureSettings, style.fontVariationSettings, tabSize, horizontalPadding, spaceWidth]),
			font,
			fontSize,
			letterSpacing,
			spaceWidth,
			tabSize,
			horizontalPadding,
			contentLeftPadding,
		});
	}

	private measureSegment(text: string): number {
		if (!text) return 0;
		const characterCount = [...text].length;
		const width = this.context?.measureText(text).width ?? characterCount * this.metrics.fontSize * 0.6;
		return Math.max(0, width + characterCount * this.metrics.letterSpacing);
	}
}

function cssNumber(value: string, fallback: number): number {
	const parsed = Number.parseFloat(value);
	return Number.isFinite(parsed) ? parsed : fallback;
}

function positiveNumber(value: number | undefined, fallback: number): number {
	return value !== undefined && Number.isFinite(value) && value > 0 ? value : fallback;
}

function validateClientPoint(point: ClientPoint): void {
	if (
		!point ||
		!isFiniteNumber(point.clientX) ||
		!isFiniteNumber(point.clientY)
	) {
		throw new RangeError(
			"Stanza client point must contain finite coordinates",
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
	if (!isFiniteNumber(value) || value < 0) {
		throw new RangeError(`Stanza editor padding.${side} must be non-negative and finite`);
	}
	return value;
}
