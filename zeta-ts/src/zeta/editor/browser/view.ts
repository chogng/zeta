import { type Event } from '../../base/common/event.js';
import { getClientArea, type IDimension } from '../../base/browser/geometry.js';
import { addDisposableListener, h } from '../../base/browser/dom.js';
import { FastDomNode } from '../../base/browser/fastDomNode.js';
import { Disposable, type IDisposable, toDisposable } from '../../base/common/lifecycle.js';
import { runWhenWindowIdle } from '../../base/browser/scheduler.js';
import { type ISize } from '../../base/common/layout.js';
import { clamp, isFiniteNumber, isNonNegativeSafeInteger } from '../../base/common/numbers.js';
import { type IAccessibilityService } from '../../platform/accessibility/common/accessibility.js';
import { type EditorSelectionController } from '../common/cursor/editorSelectionController.js';
import { resolveEditorIndentationOptions, type EditorIndentationOptions, type ResolvedEditorIndentationOptions } from '../common/editorIndentation.js';
import { TextPosition, type TextRange } from '../common/core/text.js';
import { type TextModel } from '../common/model/textModel.js';
import { type EditorVisualLineProjection } from '../common/viewModel/modelLineProjection.js';
import { type EditorScrollPosition } from '../common/viewModel.js';
import { EditorLineWrapping, isWrappingIndent, WrappingIndent } from '../common/config/editorOptions.js';
import { type EditorLineVisibilitySource, ViewModelLines } from '../common/viewModel/viewModelLines.js';
import { type EditorViewportChange, type EditorViewportLayout, ViewLayout } from '../common/viewLayout/viewLayout.js';
import { CompositionController } from './controller/compositionController.js';
import { createEditContext } from './controller/editContext/factory.js';
import { type EditContext, type EditContextCharacterBounds } from './controller/editContext/editContext.js';
import { NativeEditContext } from './controller/editContext/native/nativeEditContext.js';
import { ScreenReaderSupport } from './controller/editContext/native/screenReaderSupport.js';
import { TextAreaAccessibilityController } from './controller/editContext/textArea/textAreaAccessibilityController.js';
import { TextAreaEditContext } from './controller/editContext/textArea/textAreaEditContext.js';
import { ViewController, type EditorCommandContext, type EditorCommandTransformer, type EditorLanguageEditingAdapter, type EditorViewDidEditEvent, type EditorViewTextUpdateEvent } from './view/viewController.js';
import { type ClientPoint, type EditorHitTarget, EditorHitTargetKind, hitTestStanzaVisualEditorPoint } from '../common/viewModel/pointerHitTest.js';
import { applyEditorFontInfo } from './config/domFontInfo.js';
import { ElementSizeObserver } from './config/elementSizeObserver.js';
import { DomTextMeasurer, type TextMeasurer } from './config/fontMeasurements.js';
import { LineWidthIndex } from './measurement/lineWidthIndex.js';
import { type DecorationSource } from './viewparts/decorations/decorationPresentation.js';
import { getStanzaDomTextCaretLeft, getStanzaDomTextOffsetAtClientPoint } from './viewparts/viewportOverlay/domTextGeometry.js';
import { type BracketColorizationSource, type SemanticTokenSource } from './viewparts/semanticTokens/semanticTokenPresentation.js';
import { type ActiveLineHighlight, type ViewportOverlayContext } from './viewparts/viewportOverlay/viewportOverlayPresentation.js';
import { getTextGraphemeBoundaries } from '../common/core/textSegmentation.js';
import { type EditorLineGutterDecoration } from './viewparts/margin/lineGutterDecoration.js';
import { MarginPart } from './viewparts/margin/marginPart.js';
import { RulersPart, type EditorRuler } from './viewparts/rulers/rulersPart.js';
import { EditorScrollbarPart } from './viewparts/editorScrollbar/editorScrollbarPart.js';
import { LineNumbersPart } from './viewparts/lineNumbers/lineNumbersPart.js';
import { MinimapPart } from './viewparts/minimap/minimapPart.js';
import { OverviewRulerPart } from './viewparts/overviewRuler/overviewRulerPart.js';
import { ScrollDecorationPart } from './viewparts/scrollDecoration/scrollDecorationPart.js';
import { EditorViewContext, EditorViewPartCollection } from './view/viewPart.js';
import { ViewOverlays } from './view/viewOverlays.js';
import { ViewLinesPart } from './viewparts/viewLines/viewLinesPart.js';
import { createEditorRenderingContext, createEditorViewportData, type EditorRenderingContext } from './view/renderingContext.js';
import { ViewUserInputEvents } from './view/viewUserInputEvents.js';
import { DOMLineBreaksComputer } from './view/domLineBreaksComputer.js';
import './media/editorViewport.css';

export type EditorViewViewportOptions = Omit<EditorViewportOptions, 'container' | 'model' | 'lineHeight' | 'ariaLabel' | 'selectionController'>;

export type { EditorCommandContext, EditorCommandTransformer, EditorLanguageEditingAdapter, EditorLanguageTypeCommand, EditorViewDidEditEvent, EditorViewTextUpdateEvent } from './view/viewController.js';
export { ViewUserInputEvents } from './view/viewUserInputEvents.js';
export type { EventCallback, EditorViewMouseTargetKind, EditorViewMouseTarget, EditorViewMouseEvent, EditorViewPartialMouseEvent } from './view/viewUserInputEvents.js';

export interface EditorViewOptions {
	readonly container: HTMLElement;
	readonly model: TextModel;
	readonly selectionController: EditorSelectionController;
	readonly lineHeight: number;
	readonly ariaLabel?: string;
	/** Stable identity used by host code that needs to address this view. */
	readonly ownerId?: string;
	readonly viewport?: EditorViewViewportOptions;
	readonly accessibilityService?: IAccessibilityService;
	readonly renderRichScreenReaderContent?: boolean;
	readonly accessibilityPageSize?: number;
	readonly semanticTokenSource?: SemanticTokenSource;
	readonly bracketColorizationSource?: BracketColorizationSource;
	/** Language-aware typing is supplied by an editor contribution, not the view itself. */
	readonly languageEditing?: EditorLanguageEditingAdapter;
	readonly wordPattern?: () => RegExp | undefined;
	/** Optional view-input bridge; the view creates one when omitted. */
	readonly userInputEvents?: ViewUserInputEvents;
}

/**
 * The browser view/input boundary for one line editor.
 *
 * This follows the VS Code split: the view selects and owns the concrete
 * EditContext adapters own browser input, while ViewController routes semantic
 * input into common commands.
 * View owns DOM projection and rendering; feature contributions own
 * policies such as completion.
 */
export class EditorView extends Disposable {
	readonly ownerId: string;
	readonly viewport: View;
	readonly selectionController: EditorSelectionController;
	readonly editContext: EditContext;
	/** Compatibility alias for integrations that call the browser surface input. */
	readonly input: EditContext;
	readonly element: HTMLElement;
	readonly textArea: HTMLTextAreaElement | undefined;
	readonly compositionController: CompositionController;
	readonly viewController: ViewController;
	readonly userInputEvents: ViewUserInputEvents;
	readonly onWillBeforeInput: Event<InputEvent>;
	readonly onWillTextUpdate: Event<EditorViewTextUpdateEvent>;
	readonly onWillKeydown: Event<KeyboardEvent>;
	readonly onDidEdit: Event<EditorViewDidEditEvent>;

	constructor(options: EditorViewOptions);
	/** Test and low-level integration overload for an already-created viewport. */
	constructor(viewport: View, selectionController: EditorSelectionController, options?: Pick<EditorViewOptions, 'ariaLabel' | 'accessibilityService' | 'renderRichScreenReaderContent' | 'accessibilityPageSize' | 'semanticTokenSource' | 'bracketColorizationSource' | 'languageEditing' | 'wordPattern' | 'userInputEvents'>);
	constructor(
		optionsOrViewport: EditorViewOptions | View,
		legacySelectionController?: EditorSelectionController,
		legacyOptions?: Pick<EditorViewOptions, 'ariaLabel' | 'accessibilityService' | 'renderRichScreenReaderContent' | 'accessibilityPageSize' | 'semanticTokenSource' | 'bracketColorizationSource' | 'languageEditing' | 'wordPattern' | 'userInputEvents'>,
	) {
		super();
		try {
			const existingViewport = optionsOrViewport instanceof View ? optionsOrViewport : undefined;
			const options = existingViewport ? undefined : optionsOrViewport as EditorViewOptions;
			const selectionController = existingViewport ? legacySelectionController : options!.selectionController;
			if (!selectionController) throw new TypeError('Editor view requires a selection controller');
			this.selectionController = selectionController;
			this.ownerId = options?.ownerId === undefined ? nextEditorViewId() : validateOwnerId(options.ownerId);
			this.viewport = existingViewport
				? existingViewport
				: this._register(new View({
					...options!.viewport,
					container: options!.container,
					model: options!.model,
					lineHeight: options!.lineHeight,
					ariaLabel: options!.ariaLabel,
					selectionController,
				}));
			const viewOptions = existingViewport ? legacyOptions ?? {} : options!;
			validateViewOptions(viewOptions);
			this.userInputEvents = viewOptions.userInputEvents ?? new ViewUserInputEvents();
			if (this.viewport.textModel !== selectionController.textModel) {
				throw new TypeError('Editor view and selection controller must share one text model');
			}
			if (viewOptions.languageEditing && viewOptions.languageEditing.textModel !== this.viewport.textModel) {
				throw new TypeError('Editor view language editing must share its text model');
			}
			if (viewOptions.semanticTokenSource && viewOptions.semanticTokenSource.textModel !== this.viewport.textModel) {
				throw new TypeError('Editor view semantic tokens must share its text model');
			}
			if (viewOptions.bracketColorizationSource && viewOptions.bracketColorizationSource.textModel !== this.viewport.textModel) {
				throw new TypeError('Editor view bracket colorization must share its text model');
			}

			// Language editing is contribution-owned. The view only borrows the adapter
			// while ViewController invokes it for the current input command.
			const languageEditing = viewOptions.languageEditing;
			this.editContext = this._register(createEditContext(this.viewport.element, {
				ariaLabel: viewOptions.ariaLabel,
				readOnly: selectionController.readOnly,
				textDirection: this.viewport.editorTextDirection,
				ownerId: this.ownerId,
				characterBoundsProvider: modelOffset => this.characterBoundsAt(modelOffset),
			}));
			this.input = this.editContext;
			this.element = this.editContext.element;
			this.textArea = this.editContext instanceof TextAreaEditContext
				? this.editContext.element
				: undefined;
			this.compositionController = this._register(new CompositionController(
				this.editContext,
				this.viewport,
				selectionController,
			));
			this.viewController = this._register(new ViewController(
				this.viewport,
				selectionController,
				{ languageEditing, wordPattern: viewOptions.wordPattern, userInputEvents: this.userInputEvents },
			));
			this.editContext.connectViewController(this.viewController, this.compositionController);
			this.onWillBeforeInput = this.editContext.onWillBeforeInput;
			this.onWillTextUpdate = this.editContext.onWillTextUpdate;
			this.onWillKeydown = this.editContext.onWillKeydown;
			this.onDidEdit = this.viewController.onDidEdit;
			this._register(this.viewController.onDidChangeOvertype(overtyping => {
				this.viewport.element.classList.toggle('overtype', overtyping);
			}));

			if (this.editContext instanceof NativeEditContext) {
				this._register(new ScreenReaderSupport({
					element: this.editContext.element,
					model: this.viewport.textModel,
					viewport: this.viewport,
					selectionController,
					onDidFocus: this.editContext.onDidFocus,
					onDidBlur: this.editContext.onDidBlur,
					accessibilityService: viewOptions.accessibilityService,
					renderRichContent: viewOptions.renderRichScreenReaderContent,
					accessibilityPageSize: viewOptions.accessibilityPageSize,
					semanticTokenSource: viewOptions.semanticTokenSource,
					bracketColorizationSource: viewOptions.bracketColorizationSource,
					isComposing: () => this.compositionController.composing,
				}));
			}
			this._register(this.compositionController.onDidChange(composing => {
				if (!composing) this.synchronizeEditContext();
			}));
			if (this.editContext instanceof TextAreaEditContext) {
				this._register(new TextAreaAccessibilityController(
					this.editContext,
					this.viewport,
					selectionController,
					this.compositionController,
				));
			}
			this._register(toDisposable(() => {
				this.viewport.element.classList.remove('input-focused');
				this.viewport.element.classList.remove('overtype');
			}));
			this._register(addDisposableListener(this.viewport.element, 'focus', event => {
				if (event.target === this.viewport.element) this.focus();
			}));
			this._register(this.editContext.onDidFocus(() => {
				this.viewport.element.classList.add('input-focused');
			}));
			this._register(this.editContext.onDidBlur(() => {
				this.viewport.element.classList.remove('input-focused');
				this.editContext.clear();
			}));
			this._register(selectionController.onDidChange(() => this.synchronizeEditContext()));
			this._register(this.viewport.textModel.onDidChange(() => this.synchronizeEditContext()));
			this.synchronizeEditContext();
			this.editContext.connect();
		} catch (error) {
			this.dispose();
			throw error;
		}
	}

	get viewportElement(): HTMLDivElement {
		return this.viewport.element;
	}

	layout(dimension: IDimension = getClientArea(this.viewport.element)): void {
		this.viewport.layout({
			width: Math.max(0, dimension.width),
			height: Math.max(0, dimension.height),
		});
	}

	focus(): void {
		this.editContext.focus();
	}

	get overtyping(): boolean {
		return this.viewController.overtyping;
	}

	registerCommandTransformer(transformer: EditorCommandTransformer): IDisposable {
		return this.viewController.registerCommandTransformer(transformer);
	}

	/** Toggles this editor view's transient overtype mode. */
	toggleOvertype(): boolean {
		return this.viewController.toggleOvertype();
	}

	/** Reveals a model position for an input contribution after it commits an edit. */
	revealPosition(position: Parameters<View['revealPosition']>[0]): void {
		this.viewport.revealPosition(position);
	}

	clearInput(): void {
		this.editContext.clear();
	}

	private synchronizeEditContext(): void {
		const selection = this.selectionController.selections.primary;
		this.editContext.syncState({
			text: this.viewport.textModel.getText(),
			selectionStart: this.viewport.textModel.offsetAt(selection.range.start),
			selectionEnd: this.viewport.textModel.offsetAt(selection.range.end),
		});
		this.editContext.updateBounds(
			this.viewport.getPositionContentCoordinates(selection.active),
		);
	}

	private characterBoundsAt(modelOffset: number): EditContextCharacterBounds | undefined {
		const model = this.viewport.textModel;
		if (!Number.isSafeInteger(modelOffset) || modelOffset < 0 || modelOffset >= model.length) return undefined;
		const position = model.positionAt(modelOffset);
		const nextPosition = model.positionAt(Math.min(model.length, modelOffset + 1));
		const start = this.viewport.getPositionContentCoordinates(position);
		const end = this.viewport.getPositionContentCoordinates(nextPosition);
		const width = position.lineIndex === nextPosition.lineIndex
			? Math.max(1, Math.abs(end.left - start.left))
			: Math.max(1, this.viewport.measureTextWidth(' '));
		return Object.freeze({
			left: Math.min(start.left, end.left),
			top: start.top,
			width,
			height: start.height,
		});
	}
}

let nextViewId = 1;

function nextEditorViewId(): string {
	return `stanza-editor-view-${nextViewId++}`;
}

function validateOwnerId(ownerId: string): string {
	if (typeof ownerId !== 'string' || ownerId.trim().length === 0) {
		throw new TypeError('Editor view ownerId must be a non-empty string');
	}
	return ownerId;
}

function validateViewOptions(options: Pick<EditorViewOptions, 'accessibilityPageSize'>): void {
	validateAccessibilityPageSize(options.accessibilityPageSize);
}

function validateAccessibilityPageSize(pageSize: number | undefined): void {
	if (pageSize !== undefined && (!Number.isSafeInteger(pageSize) || pageSize < 1 || pageSize > 10_000)) {
		throw new RangeError('Editor accessibility page size must be a safe integer between 1 and 10000');
	}
}
export type EditorViewportPresentation = "document" | "embedded";

/** Chooses which component renders the visible focus outline for an Stanza viewport. */
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

export type { EditorRuler } from "./viewparts/rulers/rulersPart.js";

/** Controls the browser paragraph direction used to shape Stanza's rendered text. */
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
	readonly wrappingIndent?: WrappingIndent;
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
	private readonly viewport: ViewLayout;
	private readonly contentElement: HTMLDivElement;
	private readonly contentNode: FastDomNode<HTMLDivElement>;
	private readonly textMetricsElement: HTMLSpanElement;
	private readonly accessibilityStatusElement: HTMLDivElement;
	private readonly viewContext: EditorViewContext;
	private readonly viewParts: EditorViewPartCollection;
	private readonly viewLinesPart: ViewLinesPart;
	private readonly marginPart: MarginPart;
	private readonly viewOverlays: ViewOverlays;
	private readonly textMeasurer: TextMeasurer;
	private readonly lineWidths: LineWidthIndex;
	private readonly viewModelLines: ViewModelLines;
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
	private readonly elementSizeObserver: ElementSizeObserver;
	private softWrapping: boolean;

	get currentLayout(): EditorViewportLayout {
		return this.viewport.layout;
	}

	constructor(options: EditorViewportOptions) {
		super();
		const ownerDocument = options.container.ownerDocument;
		this.model = options.model;
		this.element = h(ownerDocument, "div");
		this.elementSizeObserver = this._register(new ElementSizeObserver(this.element));
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
				throw new TypeError("Unknown Stanza editor minimap mode");
			}
			if (!Object.values(EditorTextDirection).includes(this.textDirection)) {
				throw new TypeError("Unknown Stanza editor text direction");
			}
			if (this.focusOutlineOwner !== "editor" && this.focusOutlineOwner !== "host") {
				throw new TypeError("Unknown Stanza editor focus outline owner");
			}
			if (this.activeLineHighlight !== "on" && this.activeLineHighlight !== "off") {
				throw new TypeError("Unknown Stanza editor active-line highlight");
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
		this.lineGutterDecoration = options.lineGutterDecoration ? this._register(options.lineGutterDecoration) : undefined;
		this.element.className = "stanza-editor";
		this.element.classList.add(`stanza-editor-${this.presentation}`);
		this.element.classList.add(`stanza-editor-focus-owner-${this.focusOutlineOwner}`);
		this.element.classList.add(`stanza-editor-direction-${this.textDirection}`);
		this.element.classList.toggle("hide-line-numbers", !this.showLineNumbers);
		applyEditorFontInfo(this.element, {
			fontFamily: options.fontFamily,
			fontSize: options.fontSize,
			fontLigatures: options.fontLigatures ?? false,
		});
		this.element.style.tabSize = String(this.indentation.tabSize);
		this.element.style.setProperty("--stanza-editor-padding-left", `${this.padding.left}px`);
		this.element.style.setProperty("--stanza-editor-padding-right", `${this.padding.right}px`);
		this.element.dir = this.textDirection;
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
		this.lineWidths = this._register(new LineWidthIndex(
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
		this.viewModelLines = this._register(new ViewModelLines(
			this.model,
			new DOMLineBreaksComputer(this.textMeasurer, this.indentation.tabSize),
			{
				wrapping: options.lineWrapping,
				wrappingIndent: options.wrappingIndent,
				initialWrappingMeasurement: {
					schedule: callback => runWhenWindowIdle(
						ownerDocument.defaultView!,
						() => callback(),
						{ timeoutMs: 250 },
					),
				},
				visibilitySource: options.lineVisibilitySource,
			},
		));
		const viewport = this._register(new ViewLayout(this.model, {
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
		this.viewLinesPart = this._register(new ViewLinesPart({
			host: this.contentElement,
			model: this.model,
			readVisualProjection: () => this.visualProjection,
			readProjectionRevision: () => this.viewModelLines.revision,
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
		this.viewOverlays = this._register(new ViewOverlays(this.viewContext, {
			contentElement: this.contentElement,
			model: this.model,
			selectionController: this.selectionController,
			decorationSources: options.decorationSources ?? [],
			showIndentationGuides: this.showIndentationGuides,
			indentationTabSize: this.indentation.tabSize,
		}));
		const rulersPart = this.viewParts.register(new RulersPart({
			host: this.contentElement,
			textMeasurer: this.textMeasurer,
			readTextLeft: () => this.textLeft,
			rulers: options.rulers,
		}));
		this.viewParts.register(new EditorScrollbarPart({
			container: this.element,
			viewport: this.element,
			scrollTo: position => this.scrollTo(position),
		}));
		const minimapPart = this.viewParts.register(new MinimapPart({
			host: this.element,
			model: this.model,
			readLayout: () => this.viewport.layout,
			readRenderingContext: () => this.createRenderingContext(this.viewport.layout),
			scrollTo: position => this.scrollTo(position),
			readMarkers: () => this.viewOverlays.decorationsPart.minimapMarkers(),
			readMarkersRevision: () => this.viewOverlays.decorationsPart.markersRevision,
			enabled: this.minimap === EditorMinimap.On,
		}));
		const overviewRulerPart = this.viewParts.register(new OverviewRulerPart({
			host: this.element,
			minimapEnabled: this.minimap === EditorMinimap.On,
			readLineCount: () => this.model.lineCount,
			readMarkers: () => this.viewOverlays.decorationsPart.overviewMarkers(),
			readMarkersRevision: () => this.viewOverlays.decorationsPart.markersRevision,
		}));
		const scrollDecorationPart = this.viewParts.register(new ScrollDecorationPart(this.element));

		// Root order is the visual stacking contract; Parts own nodes but do not choose their host.
		this.contentElement.append(
			this.viewLinesPart.domNode,
			this.marginPart.domNode,
			this.viewOverlays.blockDecorationsPart.domNode,
			rulersPart.domNode,
		);
		this.element.append(
			minimapPart.domNode,
			overviewRulerPart.domNode,
			scrollDecorationPart.domNode,
		);
		this._register(this.viewModelLines.onDidChange(() => this.project(viewport.layout)));
		if (this.lineGutterDecoration) this._register(this.lineGutterDecoration.onDidChange(() => this.project(viewport.layout)));
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
		this._register(this.model.onDidChange(change => {
			this.lineWidths.applyModelChange(change);
			if (this.softWrapping) this.updateWrapWidth(this.viewport.layout.viewportSize.width);
			viewport.setContentWidth(this.measuredContentWidth);
		}));
		if (this.selectionController) {
			this._register(this.selectionController.onDidChange(() => {
				const context = this.createRenderingContext(viewport.layout);
				this.viewOverlays.renderSelection(context);
				this.updateAccessibilityStatus();
			}));
			this.updateAccessibilityStatus();
		}
		const semanticTokenSource = options.semanticTokenSource;
		if (semanticTokenSource) {
			this._register(semanticTokenSource.onDidChange(() => {
				this.viewLinesPart.renderVisibleLineText();
			}));
		}
		const fontSet = ownerDocument.fonts;
		if (fontSet) {
			this._register(addDisposableListener(fontSet, "loadingdone", () => {
				this.refreshFontMetrics();
			}));
		}

		this._register(this.elementSizeObserver.onDidChange(() => this.layout()));
		this.elementSizeObserver.startObserving();

		this.project(viewport.layout);
		this.elementSizeObserver.observeNow();
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
		return this.textDirection;
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
		const caretLeft = domCaretLeft ?? (this.textLeft + (visualLine.wrappedTextIndentWidth ?? 0) +
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
			left: domCaretLeft ?? (this.textLeft + (visualLine.wrappedTextIndentWidth ?? 0) + this.textMeasurer.measureLineWidth(
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
		if (!isFiniteNumber(horizontalOffset)) throw new RangeError("Stanza visual cursor horizontal offset must be finite");
		if (this.textDirection === EditorTextDirection.LeftToRight) return undefined;
		const visualLine = this.visualProjection.lineAt(visualLineIndex);
		const line = this.viewLinesPart.renderedLines.get(visualLineIndex);
		if (!visualLine || !line) return undefined;
		const text = this.model.getLineContent(visualLine.logicalLineIndex).slice(visualLine.startColumn, visualLine.endColumn);
		if (line.textElement.textContent?.length !== text.length) return undefined;
		let nearestColumn: number | undefined;
		let nearestDistance = Number.POSITIVE_INFINITY;
		for (const column of getTextGraphemeBoundaries(text)) {
			const left = getStanzaDomTextCaretLeft(line.textElement, column, line.domNode.domNode);
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
		this.viewOverlays.setCompositionRange(range);
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
		return hitTestStanzaVisualEditorPoint(
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
			const offset = getStanzaDomTextOffsetAtClientPoint(
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
		return line && isNonNegativeSafeInteger(offset) && offset <= line.textElement.textContent?.length
			? getStanzaDomTextCaretLeft(line.textElement, offset, line.domNode.domNode)
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
		this.viewModelLines.setWrapWidth(Math.max(
			0,
			viewportWidth - this.gutterWidth - this.textMeasurer.horizontalPadding,
		));
	}

	private project(layout: EditorViewportLayout): void {
		this.observeRenderedLineWidths(layout);
		if (layout !== this.viewport.layout) return;
		const viewportData = createEditorViewportData(layout);
		this.viewLinesPart.render(viewportData);
		const context = this.createRenderingContext(layout, viewportData);
		this.element.classList.toggle("horizontally-scrollable", layout.maximumScrollPosition.left > 0);
		this.element.classList.toggle("vertically-scrollable", layout.maximumScrollPosition.top > 0);
		this.contentNode.setWidth(layout.contentSize.width);
		this.contentNode.setHeight(layout.contentSize.height);
		this.viewParts.prepareRender(context);
		this.viewOverlays.prepareRender(context);
		this.viewParts.render(context);
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

	private createRenderingContext(layout: EditorViewportLayout, viewportData = createEditorViewportData(layout)): EditorRenderingContext {
		const overlay: ViewportOverlayContext = {
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


export { View as EditorViewport };
