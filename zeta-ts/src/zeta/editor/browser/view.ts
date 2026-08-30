import { Event } from '../../base/common/event.js';
import { getClientArea } from '../../base/browser/dom.js';
import { addDisposableListener, h } from '../../base/browser/dom.js';
import { FastDomNode } from '../../base/browser/fastDomNode.js';
import { PixelRatio, type IPixelRatioMonitor } from '../../base/browser/pixelRatio.js';
import { Disposable, type IDisposable, toDisposable } from '../../base/common/lifecycle.js';
import { runWhenWindowIdle } from '../../base/browser/dom.js';
import { type ISize } from '../../base/common/layout.js';
import { clamp, isFiniteNumber } from '../../base/common/numbers.js';
import { type CursorsController } from '../common/cursor/cursor.js';
import { resolveEditorIndentationOptions, type EditorIndentationOptions, type ResolvedEditorIndentationOptions } from '../common/core/misc/indentation.js';
import { Position } from '../common/core/position.js';
import { type IDimension } from '../common/core/2d/dimension.js';
import { type Range } from '../common/core/range.js';
import { type TextModel } from '../common/model/textModel.js';
import { type IAttachedView } from '../common/model.js';
import { CursorConfiguration } from '../common/cursorCommon.js';
import { createBuiltinLanguageConfigurationService } from '../common/languages/languageBuiltinConfigurations.js';
import { type ILanguageConfigurationService } from '../common/languages/languageConfigurationRegistry.js';
import { type EditorVisualLineProjection } from '../common/viewModel/modelLineProjection.js';
import { type EditorScrollPosition } from '../common/viewModel/editorViewportContracts.js';
import { ComputeOptionsMemory, EditorLayoutInfoComputer, EditorLineWrapping, EditorOption, EditorOptions, type EditorMinimapLayoutInfo, type EditorMinimapOptions, type IEditorMinimapOptions, type IEditorOptions, type InternalEditorRenderLineNumbersOptions, type InternalGuidesOptions, RenderLineNumbersType, isWrappingIndent, TextEditorCursorStyle, WrappingIndent } from '../common/config/editorOptions.js';
import { EDITOR_FONT_DEFAULTS, FontInfo } from '../common/config/fontInfo.js';
import { type EditorLineVisibilitySource, ViewModelLines } from '../common/viewModel/viewModelLines.js';
import { type EditorViewportChange, type EditorViewportLayout, EditorViewportLayoutManager } from '../common/viewLayout/viewLayout.js';
import { type ClientPoint, type EditorHitTarget, EditorHitTargetKind, hitTestStanzaVisualEditorPoint } from '../common/viewModel/pointerHitTest.js';
import { applyEditorFontInfo } from './config/domFontInfo.js';
import { ElementSizeObserver } from './config/elementSizeObserver.js';
import { EditorConfiguration } from './config/editorConfiguration.js';
import { DomTextMeasurer, type TextMeasurer } from './config/fontMeasurements.js';
import { type DecorationSource } from './viewParts/decorations/decorations.js';
import { type BracketColorizationSource, type SemanticTokenSource } from './viewParts/viewLines/viewLine.js';
import { getTextGraphemeBoundaries } from '../common/core/textSegmentation.js';
import { EditorMargin } from './viewParts/margin/margin.js';
import { EditorGlyphMarginWidgets, resolveGlyphMarginLanes } from './viewParts/glyphMargin/glyphMargin.js';
import { EditorRulers, type EditorRuler } from './viewParts/rulers/rulers.js';
import { StyledRulersGpu } from './viewParts/rulersGpu/styledRulersGpu.js';
import { EditorViewportScrollbar } from './viewParts/editorScrollbar/editorScrollbar.js';
import { EditorLineNumbersOverlay } from './viewParts/lineNumbers/lineNumbers.js';
import { EditorMinimap } from './viewParts/minimap/minimap.js';
import { EditorDecorationsOverviewRuler } from './viewParts/overviewRuler/decorationsOverviewRuler.js';
import { EditorScrollDecorationViewPart } from './viewParts/scrollDecoration/scrollDecoration.js';
import { EditorContentWidgets } from './viewParts/contentWidgets/contentWidgets.js';
import { ViewOverlayWidgets } from './viewParts/overlayWidgets/overlayWidgets.js';
import { EditorViewContext, EditorViewPartCollection } from './view/viewPart.js';
import type { IContentWidget, IOverlayWidget, IViewZoneChangeAccessor } from './editorBrowser.js';
import { EditorOverlayCoordinator } from './view/editorOverlayCoordinator.js';
import { LineWidthIndex, EditorViewLines } from './viewParts/viewLines/viewLines.js';
import { EditorTextDirection, EditorViewLineOptions } from './viewParts/viewLines/viewLineOptions.js';
import { StyledViewLinesGpu } from './viewParts/viewLinesGpu/styledViewLinesGpu.js';
import { EditorViewZones, type EditorViewZone, type EditorViewZoneHandle } from './viewParts/viewZones/viewZones.js';
import { linesDecorationsWidth } from './viewParts/linesDecorations/linesDecorations.js';
import { createEditorRenderingContext, createEditorViewportData, type EditorOverlayContext, type EditorRenderingContext } from './view/renderingContext.js';
import { DOMLineBreaksComputerFactory } from './view/domLineBreaksComputer.js';
import './widget/codeEditor/editor.css';

const DEFAULT_EDITOR_SCROLLBAR = EditorOptions.scrollbar.defaultValue;

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
	readonly model: TextModel;
	readonly lineHeight: number;
	readonly padding?: EditorViewportPadding;
	readonly overscanLineCount?: number;
	readonly ariaLabel?: string;
	readonly textMeasurer?: TextMeasurer;
	readonly selectionController?: CursorsController;
	readonly decorationSources?: readonly DecorationSource[];
	readonly semanticTokenSource?: SemanticTokenSource;
	readonly bracketColorizationSource?: BracketColorizationSource;
	readonly lineVisibilitySource?: EditorLineVisibilitySource;
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
	readonly languageConfigurationService?: ILanguageConfigurationService;
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
export class View extends Disposable {
	readonly element: HTMLDivElement;
	readonly onDidChangeLayout: Event<EditorViewportChange>;
	private readonly model: TextModel;
	private readonly viewport: EditorViewportLayoutManager;
	private readonly contentElement: HTMLDivElement;
	private readonly contentNode: FastDomNode<HTMLDivElement>;
	private readonly textMetricsElement: HTMLSpanElement;
	private readonly accessibilityStatusElement: HTMLDivElement;
	private readonly viewContext: EditorViewContext;
	private readonly viewParts: EditorViewPartCollection;
	private readonly viewLines: EditorViewLines;
	private readonly viewLinesGpu: StyledViewLinesGpu | undefined;
	private readonly viewZones: EditorViewZones;
	private readonly contentWidgets: EditorContentWidgets;
	private readonly overlayWidgets: ViewOverlayWidgets;
	private readonly margin: EditorMargin;
	private readonly viewOverlays: EditorOverlayCoordinator;
	private readonly textMeasurer: TextMeasurer;
	private readonly lineWidths: LineWidthIndex;
	private readonly viewModelLines: ViewModelLines;
	readonly coordinatesConverter: ReturnType<ViewModelLines['createCoordinatesConverter']>;
	readonly cursorConfig: CursorConfiguration;
	private readonly attachedView: IAttachedView;
	private readonly selectionController: CursorsController | undefined;
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
	private readonly viewLineOptions: EditorViewLineOptions;
	private readonly elementSizeObserver: ElementSizeObserver;
	private readonly pixelRatio: IPixelRatioMonitor;
	private overlayWidgetsMinimumContentWidth = 0;
	private viewZonesMinimumContentWidth = 0;
	private softWrapping: boolean;

	get currentLayout(): EditorViewportLayout {
		return this.viewport.layout;
	}

	constructor(options: EditorViewportOptions) {
		super();
		const ownerDocument = options.container.ownerDocument;
		const ownerWindow = ownerDocument.defaultView;
		if (!ownerWindow) throw new ReferenceError('Editor viewport requires a browser window');
		this.pixelRatio = PixelRatio.getInstance(ownerWindow);
		this.model = options.model;
		this.element = h(ownerDocument, "div");
		this.elementSizeObserver = this._register(new ElementSizeObserver(this.element, undefined));
		this.contentElement = h(ownerDocument, "div");
		this.contentNode = new FastDomNode(this.contentElement);
		this.textMetricsElement = h(ownerDocument, "span");
		this.accessibilityStatusElement = h(ownerDocument, "div");
		this.selectionController = options.selectionController;
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
			this.viewLineOptions = new EditorViewLineOptions({
				textDirection: options.textDirection ?? EditorTextDirection.Auto,
				fontLigatures: options.fontLigatures ?? false,
				useGpu: options.experimentalGpuAcceleration === 'on',
				useMonospaceOptimizations: false,
				lineHeight: options.lineHeight,
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
			if (this.selectionController && this.selectionController.textModel !== this.model) {
				throw new TypeError(
					"Stanza viewport and selection controller must share one text model",
				);
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
		this.element.className = "stanza-editor";
		this.element.classList.add(`stanza-editor-${this.presentation}`);
		this.element.classList.add(`stanza-editor-focus-owner-${this.focusOutlineOwner}`);
		this.element.classList.add(`stanza-editor-direction-${this.viewLineOptions.textDirection}`);
		this.element.classList.add(`stanza-editor-mouse-${mouseStyle}`);
		this.element.classList.toggle("hide-line-numbers", this.lineNumbers.renderType === RenderLineNumbersType.Off);
		applyEditorFontInfo(this.element, {
			fontFamily: options.fontFamily,
			fontSize: options.fontSize,
			fontLigatures: this.viewLineOptions.fontLigatures,
		});
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
			new DomTextMeasurer(this.textMetricsElement);
		const spaceWidth = Math.max(1, this.textMeasurer.measureLineWidth(' '));
		const typicalHalfwidthCharacterWidth = Math.max(1, this.textMeasurer.measureLineWidth('n'));
		const fontInfo = new FontInfo({
			pixelRatio: this.pixelRatio.value,
			fontFamily: options.fontFamily ?? EDITOR_FONT_DEFAULTS.fontFamily,
			fontWeight: EDITOR_FONT_DEFAULTS.fontWeight,
			fontSize: options.fontSize ?? EDITOR_FONT_DEFAULTS.fontSize,
			fontFeatureSettings: this.viewLineOptions.fontLigatures ? 'normal' : 'none',
			fontVariationSettings: 'normal',
			lineHeight: options.lineHeight,
			letterSpacing: 0,
			isMonospace: false,
			typicalHalfwidthCharacterWidth,
			typicalFullwidthCharacterWidth: Math.max(typicalHalfwidthCharacterWidth, this.textMeasurer.measureLineWidth('ｍ')),
			canUseHalfwidthRightwardsArrow: true,
			spaceWidth,
			middotWidth: this.textMeasurer.measureLineWidth('·'),
			wsmiddotWidth: this.textMeasurer.measureLineWidth('･'),
			maxDigitWidth: Math.max(...'0123456789'.split('').map(digit => this.textMeasurer.measureLineWidth(digit))),
		}, true);
		const languageConfigurationService = options.languageConfigurationService ?? this._register(createBuiltinLanguageConfigurationService());
		const editorConfiguration = this._register(new EditorConfiguration(options.cursorOptions ?? {}, fontInfo, options.container));
		this.cursorConfig = new CursorConfiguration(options.languageId ?? this.model.getLanguageId(), this.model.getOptions(), editorConfiguration, languageConfigurationService);
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
		this.viewModelLines = this._register(new ViewModelLines(
			this.model,
			new DOMLineBreaksComputerFactory(new WeakRef(ownerWindow), this.textMeasurer),
			fontInfo,
			this.indentation.tabSize,
			{
				wrapping: options.lineWrapping,
				wrappingIndent: options.wrappingIndent,
				initialWrappingMeasurement: {
					schedule: callback => runWhenWindowIdle(
						ownerWindow,
						() => callback(),
						250,
					),
				},
				visibilitySource: options.lineVisibilitySource,
			},
		));
		this.coordinatesConverter = this.viewModelLines.createCoordinatesConverter();
		this.attachedView = this.model.onBeforeAttached();
		this._register(toDisposable(() => this.model.onBeforeDetached(this.attachedView)));
		const viewport = this._register(new EditorViewportLayoutManager(this.model, {
			lineHeight: options.lineHeight,
			overscanLineCount: options.overscanLineCount,
			lineSource: this.viewModelLines.lineSource,
			padding: { top: this.padding.top, bottom: this.padding.bottom },
		}));
		this.viewport = viewport;
		this.onDidChangeLayout = viewport.onDidChange;
		this.viewContext = new EditorViewContext(
			() => viewport.layout,
			layout => this.createRenderingContext(layout),
		);
		this.viewParts = this._register(new EditorViewPartCollection());
		this.viewZones = this.viewParts.register(new EditorViewZones({
			host: this.element,
			viewLayout: this.viewport,
			readVisualLineCount: () => this.visualProjection.visualLineCount,
			readContentLeft: () => this.contentOffsetLeft + this.textLeft,
			readContentWidth: () => Math.max(0, this.viewport.layout.viewportSize.width - this.contentOffsetLeft - this.textLeft),
			setMinimumContentWidth: width => this.setViewZonesMinimumContentWidth(width),
		}));
		this.contentWidgets = this.viewParts.register(new EditorContentWidgets({
			viewDomNode: this.element,
			allowOverflow: options.allowOverflow ?? true,
			fixedOverflowWidgets: options.fixedOverflowWidgets ?? false,
			readContentLeft: () => this.contentOffsetLeft,
			readContentWidth: () => Math.max(0, this.viewport.layout.viewportSize.width - this.contentOffsetLeft),
		}));
		this.overlayWidgets = this.viewParts.register(new ViewOverlayWidgets({
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
		this.viewLines = this._register(new EditorViewLines({
			host: this.contentElement,
			model: this.model,
			readVisualProjection: () => this.visualProjection,
			readProjectionRevision: () => this.viewModelLines.revision,
			semanticTokenSource: options.semanticTokenSource,
			bracketColorizationSource: options.bracketColorizationSource,
			viewLineOptions: this.viewLineOptions,
			typicalHalfwidthCharacterWidth: Math.max(1, this.textMeasurer.measureLineWidth(' ')),
		}));
		this.viewLinesGpu = this.viewLineOptions.useGpu
			? this._register(new StyledViewLinesGpu({
				host: this.element,
				model: this.model,
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
		this.viewOverlays = this._register(new EditorOverlayCoordinator(this.viewContext, {
			contentElement: this.contentElement,
			model: this.model,
			selectionController: this.selectionController,
			semanticTokenSource: options.semanticTokenSource,
			bracketColorizationSource: options.bracketColorizationSource,
			decorationSources,
			guides: this.guides,
			indentationTabSize: this.indentation.tabSize,
			renderWhitespace: options.renderWhitespace ?? 'none',
			cursorStyle: this.cursorStyle,
			cursorBlinking,
			cursorSmoothCaretAnimation,
			cursorWidth,
			cursorHeight,
			fontInfo: {
				fontFamily: options.fontFamily,
				fontSize: options.fontSize,
				fontLigatures: this.viewLineOptions.fontLigatures,
			},
			...(this.viewLinesGpu ? { readGpuLineIndexes: () => this.viewLinesGpu!.gpuLineIndexes } : {}),
		}));
		this.margin = this.viewParts.register(new EditorMargin({
			host: this.element,
			contentElement: this.contentElement,
			model: this.model,
			textMeasurer: this.textMeasurer,
			presentation: this.presentation,
			showLineNumbers: this.lineNumbers.renderType !== RenderLineNumbersType.Off,
			glyphMarginLaneCount: glyphMarginLanes.length,
			lineHeight: options.lineHeight,
			lineDecorationsWidth: linesDecorationsWidth(decorationSources),
		}));
		this.margin.domNode.append(this.viewZones.marginDomNode);
		const glyphMarginWidgets = this.viewParts.register(new EditorGlyphMarginWidgets({
			host: this.contentElement,
			lanes: glyphMarginLanes,
			decorations: this.viewOverlays.decorations,
			readVisualLines: () => this.visualProjection,
			readLeft: () => this.margin.glyphMarginLeft,
			readLaneWidth: () => this.margin.glyphMarginLaneWidth,
		}));
		const lineNumbersOverlay = this.viewParts.register(new EditorLineNumbersOverlay({
			host: this.contentElement,
			lineNumbers: this.lineNumbers,
			selectionController: this.selectionController,
			readVisualProjection: () => this.visualProjection,
		}));
		let rulersDomNode: HTMLElement | undefined;
		if (this.viewLinesGpu) {
			this.viewParts.register(new StyledRulersGpu(
				this.viewLinesGpu.gpuContext,
				Object.freeze([...(options.rulers ?? [])]),
				column => this.textLeft + this.textMeasurer.measureLineWidth('0'.repeat(column)),
			));
		} else {
			rulersDomNode = this.viewParts.register(new EditorRulers({
				host: this.contentElement,
				textMeasurer: this.textMeasurer,
				readTextLeft: () => this.textLeft,
				rulers: options.rulers,
			})).domNode;
		}
		this.viewParts.register(new EditorViewportScrollbar({
			container: this.element,
			viewport: this.element,
			scrollTo: position => this.scrollTo(position),
			horizontalScrollbarSize: DEFAULT_EDITOR_SCROLLBAR.horizontalScrollbarSize,
			verticalScrollbarSize: DEFAULT_EDITOR_SCROLLBAR.verticalScrollbarSize,
		}));
		const minimapPart = this.viewParts.register(new EditorMinimap({
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
			readProjectionRevision: () => this.viewModelLines.revision,
			scrollTo: position => this.scrollTo(position),
			readMarkers: () => this.viewOverlays.decorations.minimapMarkers(),
			readMarkersRevision: () => this.viewOverlays.decorations.markersRevision,
		}));
		const decorationsOverviewRuler = this.viewParts.register(new EditorDecorationsOverviewRuler({
			host: this.element,
			verticalScrollbarWidth: DEFAULT_EDITOR_SCROLLBAR.verticalScrollbarSize,
			getVerticalOffsetForLineIndex: lineIndex => this.viewport.getVerticalOffsetForLineIndex(
				lineIndex >= this.model.lineCount
					? this.visualProjection.visualLineCount
					: this.visualProjection.visualLineIndexAt(new Position((lineIndex) + 1, (0) + 1)),
			),
			readMarkers: () => this.viewOverlays.decorations.overviewMarkers(),
			readMarkersRevision: () => this.viewOverlays.decorations.markersRevision,
		}));
		const scrollDecoration = this.viewParts.register(new EditorScrollDecorationViewPart(this.element));

		// Root order is the visual stacking contract; Parts own nodes but do not choose their host.
		this.contentElement.append(
			this.viewLines.domNode,
			...this.viewOverlays.domNodes,
			this.contentWidgets.domNode.domNode,
			lineNumbersOverlay.domNode,
			this.margin.domNode,
			glyphMarginWidgets.domNode,
			this.viewOverlays.blockDecorations.domNode,
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
		this._register(this.viewModelLines.onDidChange(() => this.project(viewport.layout)));
		this._register(this.viewOverlays.onDidChangeDecorations(() => this.project(viewport.layout)));
		viewport.setContentWidth(this.measuredContentWidth);

		this._register(viewport.onDidChange(({ layout }) => this.project(layout)));
		this._register(this.lineWidths.onDidChange(() => {
			viewport.setContentWidth(this.measuredContentWidth);
		}));
		this._register(addDisposableListener(this.element, "scroll", () => {
			const scrollPosition = {
				left: this.element.scrollLeft,
				top: this.element.scrollTop,
			};
			const layout = viewport.setScrollPosition(scrollPosition);
			this.syncScrollPosition(layout);
		}));
		this._register(this.model.onDidChangeContent(change => {
			this.lineWidths.applyModelChange(change);
			if (this.softWrapping) this.updateWrapWidth(this.viewport.layout.viewportSize.width);
			viewport.setContentWidth(this.measuredContentWidth);
		}));
		if (this.selectionController) {
			this._register(this.selectionController.onDidChange(change => {
				const context = this.createRenderingContext(viewport.layout);
				lineNumbersOverlay.renderNow(context);
				this.viewOverlays.renderSelection(context, change.reason);
				this.updateAccessibilityStatus();
			}));
			this.updateAccessibilityStatus();
		}
		const semanticTokenSource = options.semanticTokenSource;
		if (semanticTokenSource) {
			this._register(semanticTokenSource.onDidChange(() => {
				this.viewLines.renderVisibleLineText();
				this.viewLinesGpu?.invalidateTokens();
				const context = this.createRenderingContext(this.viewport.layout);
				this.viewLinesGpu?.render(context);
				this.viewOverlays.renderCursorTokens(context);
				minimapPart.invalidateTokens();
				minimapPart.renderNow(context);
			}));
		}
		const fontSet = ownerDocument.fonts;
		if (fontSet) {
			this._register(addDisposableListener(fontSet, "loadingdone", () => {
				this.refreshFontMetrics();
			}));
		}

		this._register(this.elementSizeObserver.onDidChange(() => this.layout()));
		this._register(this.pixelRatio.onDidChange(() => this.project(viewport.layout)));
		this.elementSizeObserver.startObserving();

		this.project(viewport.layout);
		this.elementSizeObserver.observe();
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
		return this.viewModelLines.wrappingIndent;
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
		if (nextSoftWrapping) this.updateWrapWidth(this.viewport.layout.viewportSize.width);
		this.viewModelLines.setWrapping(lineWrapping);
		this.element.classList.toggle("word-wrapped", nextSoftWrapping);
		const layout = this.viewport.setContentWidth(this.measuredContentWidth);
		this.project(layout);
		return layout;
	}

	setWrappingIndent(wrappingIndent: WrappingIndent): EditorViewportLayout {
		if (!isWrappingIndent(wrappingIndent)) {
			throw new TypeError("Unknown Stanza wrapping indent mode");
		}
		if (wrappingIndent === this.wrappingIndent) return this.viewport.layout;
		this.viewModelLines.setWrappingIndent(wrappingIndent);
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
			throw new TypeError("Stanza accessibility status must be a non-empty string");
		}
		this.accessibilityStatusElement.textContent = message.trim();
	}

	refreshFontMetrics(): EditorViewportLayout {
		if (!this.textMeasurer.refresh()) return this.viewport.layout;
		this.viewLinesGpu?.invalidateFont();
		this.viewOverlays.setCursorLineWidth(Math.min(
			this.configuredCursorWidth,
			Math.max(1, this.textMeasurer.measureLineWidth(' ')),
		));
		this.lineWidths.refresh();
		if (this.softWrapping) this.updateWrapWidth(this.viewport.layout.viewportSize.width);
		const layout = this.viewport.setContentWidth(this.measuredContentWidth);
		this.project(layout);
		return layout;
	}

	setLineHeight(lineHeight: number): EditorViewportLayout {
		this.margin.setLineHeight(lineHeight);
		const lineHeightLayout = this.viewport.setLineHeight(lineHeight);
		this.viewZones.setLineHeight(lineHeight);
		if (this.softWrapping) this.updateWrapWidth(lineHeightLayout.viewportSize.width);
		const layout = this.viewport.setContentWidth(this.measuredContentWidth);
		this.project(layout);
		return layout;
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
		this.overlayWidgets.setWidgetPosition(widget, widget.getPosition());
		this.project(this.viewport.layout);
	}

	removeOverlayWidget(widget: IOverlayWidget): void {
		this.overlayWidgets.removeWidget(widget);
	}

	scrollTo(position: EditorScrollPosition): EditorViewportLayout {
		const layout = this.viewport.setScrollPosition(position);
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
		this.viewOverlays.setCompositionRange(range);
	}

	setOvertype(overtyping: boolean): void {
		this.viewOverlays.setCursorStyle(overtyping ? this.overtypeCursorStyle : this.cursorStyle);
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
		this.viewport.setContentWidth(this.measuredContentWidth);
	}

	private setViewZonesMinimumContentWidth(width: number): void {
		if (width === this.viewZonesMinimumContentWidth) return;
		this.viewZonesMinimumContentWidth = width;
		this.viewport.setContentWidth(this.measuredContentWidth);
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

	private updateWrapWidth(viewportWidth: number): void {
		const minimapWidth = this.computeMinimapLayout(viewportWidth, this.viewport.layout.viewportSize.height).minimapWidth;
		const rightControlWidth = minimapWidth > 0
			? minimapWidth + DEFAULT_EDITOR_SCROLLBAR.verticalScrollbarSize
			: 0;
		this.viewModelLines.setWrapWidth(Math.max(
			0,
			viewportWidth - this.gutterWidth - this.textMeasurer.horizontalPadding - rightControlWidth,
		));
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
		const firstVisibleLine = this.visualProjection.lineAt(layout.visibleLines.startLineIndex);
		const lastVisibleLine = this.visualProjection.lineAt(layout.visibleLines.endLineIndexExclusive - 1);
		this.attachedView.setVisibleLines(firstVisibleLine && lastVisibleLine ? [{
			startLineNumber: firstVisibleLine.logicalLineIndex + 1,
			endLineNumber: lastVisibleLine.logicalLineIndex + 1,
		}] : [], false);
		const viewportData = createEditorViewportData(layout);
		this.viewLines.render(viewportData);
		const context = this.createRenderingContext(layout, viewportData);
		this.element.classList.toggle("horizontally-scrollable", layout.maximumScrollPosition.left > 0);
		this.element.classList.toggle("vertically-scrollable", layout.maximumScrollPosition.top > 0);
		this.contentNode.setWidth(layout.contentSize.width);
		this.contentNode.setHeight(layout.contentSize.height);
		const contentOffsetLeft = this.contentOffsetLeft;
		this.contentNode.setTransform(contentOffsetLeft > 0 ? `translate3d(${contentOffsetLeft}px, 0, 0)` : '');
		this.viewParts.prepareRender(context);
		this.viewOverlays.prepareRender(context);
		this.viewParts.render(context);
		this.viewLinesGpu?.render(context);
		this.viewOverlays.render(context);
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
		const position = selection.getPosition();
		const selectedLength = this.model.offsetAt(selection.getEndPosition()) - this.model.offsetAt(selection.getStartPosition());
		if (selectionSet.selections.length > 1) {
			const totalSelectedLength = selectionSet.selections.reduce((length, current) =>
				length + this.model.offsetAt(current.getEndPosition()) - this.model.offsetAt(current.getStartPosition()), 0);
			const summary = totalSelectedLength === 0
				? `${selectionSet.selections.length} cursors`
				: `${selectionSet.selections.length} selections, ${totalSelectedLength} characters selected`;
			this.announceAccessibilityStatus(`${summary}; primary at Line ${position.lineNumber}, column ${position.column}`);
			return;
		}
		this.announceAccessibilityStatus(selectedLength === 0
			? `Line ${position.lineNumber}, column ${position.column}`
			: `Line ${position.lineNumber}, column ${position.column}, ${selectedLength} characters selected`);
	}

	private createRenderingContext(layout: EditorViewportLayout, viewportData = createEditorViewportData(layout)): EditorRenderingContext {
		const useDomTextGeometry = this.viewLineOptions.textDirection !== EditorTextDirection.LeftToRight;
		const overlay: EditorOverlayContext = {
			ownerDocument: this.element.ownerDocument,
			model: this.model,
			visualLineProjection: this.visualProjection,
			renderLines: layout.renderLines,
			textLeft: this.textLeft,
			textMeasurer: this.textMeasurer,
			renderLineHighlight: this.renderLineHighlight,
			renderLineHighlightOnlyWhenFocus: this.renderLineHighlightOnlyWhenFocus,
			linesVisibleRangesForRange: (range, includeNewLines) => useDomTextGeometry
				? this.viewLines.linesVisibleRangesForRange(range, includeNewLines)
				: undefined,
			visibleRangeForPosition: position => useDomTextGeometry
				? this.viewLines.visibleRangeForPosition(position)
				: undefined,
		};
		return createEditorRenderingContext(layout, overlay, viewportData);
	}

	private get visualProjection() {
		return this.viewModelLines.ensureCurrent();
	}

	private syncScrollPosition(layout: EditorViewportLayout): void {
		if (this.element.scrollLeft !== layout.scrollPosition.left) {
			this.element.scrollLeft = layout.scrollPosition.left;
		}
		if (this.element.scrollTop !== layout.scrollPosition.top) {
			this.element.scrollTop = layout.scrollPosition.top;
		}
	}

	get cursorModel(): ViewModelLines {
		return this.viewModelLines;
	}
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
