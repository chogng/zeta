import { type IDimension } from "../../base/browser/geometry.js";
import { isNonEmptyArray } from "../../base/common/arrays.js";
import { type Event } from "../../base/common/event.js";
import { Disposable, type IDisposable, toDisposable } from "../../base/common/lifecycle.js";
import { isFiniteNumber, isSafeInteger } from "../../base/common/numbers.js";
import type { URI } from "../../base/common/uri.js";
import { EditorSelectionController } from "../common/cursor/editorSelectionController.js";
import { TextSelection, TextSelectionSet } from "../common/core/selection.js";
import type { IPosition } from '../common/core/position.js';
import { TextPosition, type TextRange } from "../common/core/text.js";
import { type LanguageCompletionWorkerFactory } from "../common/languages/completion/languageCompletionService.js";
import { type SyntaxWorkerFactory } from "../common/languages/syntax/syntaxService.js";
import type { ILanguageFeaturesService } from '../common/services/languageFeatures.js';
import { LanguageConfigurationService, type ILanguageConfigurationService } from '../common/services/languageConfigurationService.js';
import { LanguageFeaturesService } from '../common/services/languageFeaturesService.js';
import { type TextModel } from "../common/model/textModel.js";
import { type PositionAffinity } from '../common/model.js';
import { type EditorIndentationOptions } from "../common/core/misc/indentation.js";
import { type EditorRuler, type EditorTextDirection, type EditorView, type EditorViewport, type EditorViewportPresentation } from "./view.js";
import { CodeEditorWidget, type CodeEditorViewPositionState, type CodeEditorViewSelectionState, type CodeEditorViewState } from "./widget/codeEditor/codeEditorWidget.js";
import { type EditorHitTarget } from "../common/viewModel/pointerHitTest.js";
import { type EditorLineWrapping, type IEditorMinimapOptions, type IEditorOptions, type WrappingIndent } from "../common/config/editorOptions.js";
import { type LanguageLocation } from "../contrib/gotoSymbol/common/languageNavigation.js";
import { type LanguageWorkspaceEdit } from "../common/languages/languageWorkspaceEdit.js";
import { type ILanguageDiagnosticsService } from "../common/services/languageDiagnosticsService.js";
import { isCompletionsEnablement, type CompletionsEnablement } from "../common/services/completionsEnablement.js";
import { EditorWorkerClient, type EditorWorkerFactory } from "../common/services/editorWorker.js";
import { EditorWorker } from "../common/services/editorWebWorker.js";
import { type DecorationSource, type OwnedDecorationSource } from "./viewparts/decorations/decorations.js";
import { type IInstantiationService } from "../../platform/instantiation/common/instantiation.js";
import { type IAccessibilityService } from "../../platform/accessibility/common/accessibility.js";
import { TabFocus } from "./config/tabFocus.js";
import { resolveEditorConfiguration } from "./config/editorConfiguration.js";
import { migrateOptions } from "./config/migrateOptions.js";
import { getEditorContributions, type EditorCapability, type TextEditorContributionContext } from "./editorExtensions.js";
import { type BracketColorizationSource, type SemanticTokenSource } from "./viewparts/viewLines/viewLine.js";
import { SemanticTokensStylingService } from '../common/services/semanticTokensStylingService.js';
import { type EditorLineVisibilitySource } from "../common/viewModel/viewModelLines.js";
import { type LanguageLexicalContextSource } from "../common/languages/languageLexicalContext.js";
import { LanguageEditingAdapter } from "./view/viewController.js";
import { type ICodeEditorService } from './services/codeEditorService.js';

export interface EditorContextMenuRequest {
	readonly position: TextPosition;
	readonly target: EditorHitTarget | undefined;
	readonly clientX: number;
	readonly clientY: number;
}

export enum ContentWidgetPositionPreference {
	EXACT,
	ABOVE,
	BELOW,
}

export interface IContentWidgetPosition {
	readonly position: IPosition | null;
	readonly secondaryPosition?: IPosition | null;
	readonly preference: readonly ContentWidgetPositionPreference[];
	readonly positionAffinity?: PositionAffinity;
}

export interface IContentWidgetRenderedCoordinate {
	readonly top: number;
	readonly left: number;
}

export interface IContentWidget {
	readonly allowEditorOverflow?: boolean;
	readonly useDisplayNone?: boolean;
	readonly suppressMouseDown?: boolean;
	getId(): string;
	getDomNode(): HTMLElement;
	getPosition(): IContentWidgetPosition | null;
	beforeRender?(): IDimension | null;
	afterRender?(position: ContentWidgetPositionPreference | null, coordinate: IContentWidgetRenderedCoordinate | null): void;
}

export enum OverlayWidgetPositionPreference {
	TOP_RIGHT_CORNER,
	BOTTOM_RIGHT_CORNER,
	TOP_CENTER,
}

export interface IOverlayWidgetPositionCoordinates {
	readonly top: number;
	readonly left: number;
}

export interface IOverlayWidgetPosition {
	readonly preference: OverlayWidgetPositionPreference | IOverlayWidgetPositionCoordinates | null;
	readonly stackOrdinal?: number;
}

export interface IOverlayWidget {
	readonly onDidLayout?: Event<void>;
	readonly allowEditorOverflow?: boolean;
	getId(): string;
	getDomNode(): HTMLElement;
	getPosition(): IOverlayWidgetPosition | null;
	getMinContentWidthInPx?(): number;
}

export interface IViewZone {
	afterLineIndex: number;
	heightInPixels: number;
	heightInLines?: number;
	ordinal?: number;
	minWidthInPixels?: number;
	suppressMouseDown?: boolean;
	readonly domNode: HTMLElement;
	readonly marginDomNode?: HTMLElement | null;
	onDomNodeTop?: (top: number) => void;
	onComputedHeight?: (height: number) => void;
}

export interface IViewZoneChangeAccessor {
	addZone(zone: IViewZone): string;
	removeZone(id: string): void;
	layoutZone(id: string): void;
}

export type EditorTextViewPositionState = CodeEditorViewPositionState;
export type EditorTextViewSelectionState = CodeEditorViewSelectionState;
/** JSON-safe instance state persisted by a Workbench text-editor pane. */
export type EditorTextViewState = CodeEditorViewState;

export function isEditorTextViewState(value: unknown): value is EditorTextViewState {
	if (!value || typeof value !== "object") return false;
	const state = value as Partial<EditorTextViewState>;
	if (!isNonEmptyArray(state.selections)) return false;
	if (!isSafeInteger(state.primarySelectionIndex) || state.primarySelectionIndex! < 0 || state.primarySelectionIndex! >= state.selections.length) return false;
	if (!isViewScrollPosition(state.scrollPosition)) return false;
	return state.selections.every(selection => isViewPosition(selection?.anchor) && isViewPosition(selection?.active));
}

/** Defaults applied whenever the editor-local Find and Replace widget opens. */
export interface EditorFindOptions {
	readonly seedSearchStringFromSelection?: boolean;
	readonly autoFindInSelection?: boolean;
	readonly loop?: boolean;
	readonly matchCase?: boolean;
	readonly wholeWord?: boolean;
	readonly regularExpression?: boolean;
	readonly wordSeparators?: string;
}

/** Selects the named source sections presented as editor line headers. */
export interface EditorSectionHeaderOptions {
	readonly showRegionSectionHeaders?: boolean;
	readonly showMarkSectionHeaders?: boolean;
	readonly markSectionHeaderRegex?: string;
}

/** Resource identity and presentation hints accepted by an editor browser surface. */
export interface EditorResourceInput {
	readonly resource: URI;
	readonly label?: string;
	readonly languageId?: string;
	readonly readOnly?: boolean;
	readonly initialText?: string;
}

export interface EditorBrowserOptions {
	readonly container: HTMLElement;
	readonly input: EditorResourceInput;
	readonly languageId: string;
	/** Optional stable editor identity used by browser host integrations. */
	readonly ownerId?: string;
	/** Optional host-scoped Tab-focus state shared by multiple editor instances. */
	readonly tabFocus?: TabFocus;
	/** Optional shared language registrations and providers for this editor host. */
	readonly languageFeaturesService?: ILanguageFeaturesService;
	/** Optional shared language editing configuration for this editor host. */
	readonly languageConfigurationService?: ILanguageConfigurationService;
	/** Window-scoped constructor service for runtime editor contributions. */
	readonly instantiationService?: IInstantiationService;
	/** Host-scoped registry for live code editors and resource open handlers. */
	readonly codeEditorService?: ICodeEditorService;
	/** Optional accessibility policy used by native screen-reader content. */
	readonly accessibilityService?: IAccessibilityService;
	/** Chooses line-structured content for native screen-reader projection. */
	readonly renderRichScreenReaderContent?: boolean;
	/** Controls how many logical lines one native screen-reader page exposes. */
	readonly accessibilityPageSize?: number;
	/** Optional host service that synchronizes open models and supplies push diagnostics. */
	readonly languageDiagnosticsService?: ILanguageDiagnosticsService;
	/** Caller-owned text model rendered by this editor. */
	readonly model: TextModel;
	/** Host-selected execution boundary for model-versioned editor computations. */
	readonly editorWorkerFactory?: EditorWorkerFactory;
	readonly syntaxWorkerFactory?: SyntaxWorkerFactory;
	readonly completionWorkerFactory?: LanguageCompletionWorkerFactory;
	readonly languageSupport?: IDisposable;
	readonly onDidChangeLanguageSupport?: Event<void>;
	readonly onLanguageError?: (error: unknown) => void;
	readonly indentation?: EditorIndentationOptions;
	readonly lineWrapping?: EditorLineWrapping;
	readonly wrappingIndent?: WrappingIndent;
	readonly fontFamily?: string;
	readonly fontSize?: number;
	readonly lineHeight?: number;
	readonly fontLigatures?: boolean;
	readonly minimap?: IEditorMinimapOptions;
	readonly sectionHeaders?: EditorSectionHeaderOptions | false;
	readonly renderLineHighlight?: IEditorOptions['renderLineHighlight'];
	readonly renderLineHighlightOnlyWhenFocus?: IEditorOptions['renderLineHighlightOnlyWhenFocus'];
	readonly lineNumbers?: IEditorOptions['lineNumbers'];
	readonly occurrencesHighlight?: 'off' | 'singleFile' | 'multiFile';
	readonly occurrencesHighlightDelay?: number;
	readonly selectionHighlight?: boolean;
	readonly selectionHighlightMultiline?: boolean;
	readonly selectionHighlightMaxLength?: number;
	readonly glyphMargin?: boolean;
	readonly showSymbolIcons?: boolean;
	readonly rulers?: readonly EditorRuler[];
	readonly guides?: IEditorOptions['guides'];
	readonly bracketPairColorization?: boolean;
	readonly matchBrackets?: "never" | "near" | "always";
	readonly stickyScroll?: boolean;
	readonly suggestions?: CompletionsEnablement;
	readonly inlineCompletions?: CompletionsEnablement;
	readonly parameterHints?: boolean;
	readonly inlayHints?: boolean;
	readonly codeLens?: boolean;
	readonly colorDecorators?: boolean;
	readonly colorDecoratorsActivatedOn?: 'clickAndHover' | 'click' | 'hover';
	readonly colorDecoratorsLimit?: number;
	readonly defaultColorDecorators?: 'auto' | 'always' | 'never';
	readonly formatOnSave?: boolean;
	readonly find?: EditorFindOptions;
	/** Applies a single LF at the save boundary when the document has content and no final LF. */
	readonly insertFinalNewLine?: boolean;
	/** Browser paragraph direction for this editor browser's DOM projection. */
	readonly textDirection?: EditorTextDirection;
	readonly experimentalGpuAcceleration?: IEditorOptions['experimentalGpuAcceleration'];
	readonly renderWhitespace?: IEditorOptions['renderWhitespace'];
	readonly cursorStyle?: IEditorOptions['cursorStyle'];
	readonly cursorBlinking?: IEditorOptions['cursorBlinking'];
	readonly cursorWidth?: IEditorOptions['cursorWidth'];
	readonly cursorHeight?: IEditorOptions['cursorHeight'];
	readonly allowOverflow?: IEditorOptions['allowOverflow'];
	readonly fixedOverflowWidgets?: IEditorOptions['fixedOverflowWidgets'];
	readonly presentation?: EditorViewportPresentation;
	/** Host-owned link opening callback; the editor never opens external targets directly. */
	readonly onOpenLink?: (target: string) => void | Promise<void>;
	/** Host-owned context-menu composition; the editor supplies only hit-test data. */
	readonly onShowContextMenu?: (request: EditorContextMenuRequest) => void | Promise<void>;
	/** Host-owned execution for provider commands such as code lenses. */
	readonly onExecuteEditorCommand?: (id: string, args: readonly unknown[] | undefined) => void | Promise<void>;
	/** Host-owned cross-resource navigation; same-resource reveal remains editor-owned. */
	readonly onOpenLocation?: (location: LanguageLocation) => void | Promise<void>;
	/** Host-owned multi-resource edit transaction. */
	readonly onApplyWorkspaceEdit?: (edit: LanguageWorkspaceEdit) => void | Promise<void>;
	/** Host-contributed gutter presentation; feature semantics remain outside the editor core. */
	/** Host-created decoration sources whose lifetime transfers to this editor part. */
	readonly decorationSources?: readonly OwnedDecorationSource[];
	readonly placeholder?: string;
	readonly showUnicodeHighlights?: boolean;
	readonly fontZoom?: { readonly initialScale?: number };
}

/** Browser editor contract created from one statically selected contribution bundle. */
export interface IEditorBrowser extends IDisposable {
	readonly onDidChange: Event<void>;
	readonly codeEditor: CodeEditorWidget;
	readonly viewport: EditorViewport;
	readonly selections: EditorSelectionController;
	readonly view: EditorView;
	announceAccessibilityStatus(message: string): void;
	layout(dimension: IDimension): void;
	focus(): void;
	getValue(): string;
	setValue(value: string): void;
	revealRange(range: TextRange): void;
	getViewState(): EditorTextViewState;
	restoreViewState(state: EditorTextViewState): void;
	addContentWidget(widget: IContentWidget): void;
	layoutContentWidget(widget: IContentWidget): void;
	removeContentWidget(widget: IContentWidget): void;
	addOverlayWidget(widget: IOverlayWidget): void;
	layoutOverlayWidget(widget: IOverlayWidget): void;
	removeOverlayWidget(widget: IOverlayWidget): void;
	changeViewZones(callback: (accessor: IViewZoneChangeAccessor) => void): void;
	/** Runs editor-local formatting and normalization before the host persists the model. */
	prepareSave(): Promise<void>;
}

/** Browser composition root for the line editor. */
export class EditorBrowser extends Disposable implements IEditorBrowser {
	private readonly beforeSaveHooks: Array<() => void | Promise<void>> = [];
	readonly onDidChange: Event<void>;
	readonly codeEditor: CodeEditorWidget;
	readonly viewport: EditorViewport;
	readonly selections: EditorSelectionController;
	readonly view: EditorView;

	constructor(options: EditorBrowserOptions) {
		super();
		try {
			options = migrateOptions(options);
			validateOptions(options);
			const configuration = resolveEditorConfiguration(options);
			const tabFocus = options.tabFocus ?? this._register(new TabFocus());
			const languageId = options.languageId;
			const onLanguageError = options.onLanguageError ?? reportLanguageError;
			if (options.languageSupport) this._register(options.languageSupport);
			const model = options.model;
			const editorWorker = this._register(options.editorWorkerFactory
				? options.editorWorkerFactory(model)
				: new EditorWorkerClient(model, () => new EditorWorker()));
			this.onDidChange = listener => model.onDidChange(() => listener());
			if (options.languageFeaturesService && !options.languageConfigurationService) {
				throw new TypeError('Editor language features require their language configuration service');
			}
			const languageConfigurationService = options.languageConfigurationService ?? this._register(new LanguageConfigurationService());
			const languageFeaturesService = options.languageFeaturesService ?? this._register(new LanguageFeaturesService(languageConfigurationService));
			const semanticTokensStylingService = this._register(new SemanticTokensStylingService());
			const configurations = languageConfigurationService;
			this.selections = this._register(new EditorSelectionController(
				model,
				TextSelectionSet.single(TextSelection.collapsedAt(TextPosition.at(0, 0))),
				{ readOnly: options.input.readOnly },
			));
			const contributionCapabilities = new Map<string, unknown>();
			const getCapability = <T>(capability: EditorCapability<T>): T => {
				if (!contributionCapabilities.has(capability.id)) throw new ReferenceError(`Text editor capability '${capability.id}' is unavailable`);
				return contributionCapabilities.get(capability.id) as T;
			};
			const getOptionalCapability = <T>(capability: EditorCapability<T>): T | undefined => contributionCapabilities.get(capability.id) as T | undefined;
			const provideCapability = <T>(capability: EditorCapability<T>, value: T): void => {
				if (contributionCapabilities.has(capability.id)) throw new RangeError(`Text editor capability '${capability.id}' is already provided`);
				contributionCapabilities.set(capability.id, value);
			};
			const decorationSources: DecorationSource[] = [];
			for (const source of options.decorationSources ?? []) decorationSources.push(this._register(source));
			let lineProjection: { readonly visibilitySource: EditorLineVisibilitySource } | undefined;
			let semanticTokenSource: SemanticTokenSource | undefined;
			let bracketColorizationSource: BracketColorizationSource | undefined;
			let languageLexicalContext: LanguageLexicalContextSource | undefined;
			const selectedContributions = getEditorContributions();
			for (const contribution of selectedContributions) {
				contribution.configure?.({
					kind: "text",
					options,
					model,
					editorWorker,
					languageId,
					languageFeaturesService,
					semanticTokensStylingService,
					configurations,
					selections: this.selections,
					tabFocus,
					onLanguageError,
					getCapability,
					getOptionalCapability,
					provideCapability,
					addDecorationSource: source => decorationSources.push(source),
					setLineProjection: projection => {
						if (lineProjection) throw new Error("Text editor line projection is already configured");
						lineProjection = projection;
					},
					setSemanticTokenSource: source => {
						if (semanticTokenSource) throw new Error("Text editor semantic-token source is already configured");
						semanticTokenSource = source;
					},
					setBracketColorizationSource: source => {
						if (bracketColorizationSource) throw new Error("Text editor bracket-colorization source is already configured");
						bracketColorizationSource = source;
					},
					setLanguageLexicalContext: source => {
						if (languageLexicalContext) throw new Error("Text editor lexical context is already configured");
						languageLexicalContext = source;
					},
					register: value => this._register(value),
				});
			}
			const languageEditing = this._register(new LanguageEditingAdapter(model, this.selections, languageId, configurations, languageLexicalContext, options.indentation));
			const ariaLabel = editorLabel(options.input);
			this.codeEditor = this._register(new CodeEditorWidget({
				container: options.container,
				model,
				selectionController: this.selections,
				lineHeight: configuration.lineHeight,
				ariaLabel,
				ownerId: options.ownerId,
				placeholder: options.placeholder,
				instantiationService: options.instantiationService,
				onContributionError: onLanguageError,
				viewport: {
					lineVisibilitySource: lineProjection?.visibilitySource,
					decorationSources,
					semanticTokenSource,
					bracketColorizationSource,
					lineWrapping: options.lineWrapping,
					wrappingIndent: options.wrappingIndent,
					fontFamily: configuration.fontFamily,
					fontSize: configuration.fontSize,
					fontLigatures: configuration.fontLigatures,
					lineNumbers: options.lineNumbers,
					glyphMargin: options.glyphMargin,
					rulers: options.rulers,
					guides: options.guides,
					minimap: options.minimap,
					renderLineHighlight: options.renderLineHighlight,
					renderLineHighlightOnlyWhenFocus: options.renderLineHighlightOnlyWhenFocus,
					textDirection: options.textDirection,
					experimentalGpuAcceleration: options.experimentalGpuAcceleration,
					renderWhitespace: options.renderWhitespace,
					cursorStyle: options.cursorStyle,
					cursorBlinking: options.cursorBlinking,
					cursorWidth: options.cursorWidth,
					cursorHeight: options.cursorHeight,
					allowOverflow: options.allowOverflow,
					fixedOverflowWidgets: options.fixedOverflowWidgets,
					presentation: options.presentation,
					indentation: options.indentation,
				},
				accessibilityService: options.accessibilityService,
				renderRichScreenReaderContent: options.renderRichScreenReaderContent,
				accessibilityPageSize: options.accessibilityPageSize,
				semanticTokenSource,
				bracketColorizationSource,
				languageEditing,
				wordPattern: () => configurations.getLanguageConfiguration(languageId).wordPattern,
				keyboardNavigation: {
					wordPattern: () => configurations.getLanguageConfiguration(languageId).wordPattern,
				},
				mouseHandler: {
					wordPattern: () => configurations.getLanguageConfiguration(languageId).wordPattern,
				},
			}));
			if (options.codeEditorService) this._register(options.codeEditorService.addCodeEditor(this.codeEditor));
			this.viewport = this.codeEditor.viewport;
			this.view = this.codeEditor.view;
			const installContext: TextEditorContributionContext = {
				kind: "text",
				options,
				model,
				editorWorker,
				languageId,
				languageFeaturesService,
				configurations,
				view: this.view,
				viewport: this.viewport,
				selections: this.selections,
				tabFocus,
				onLanguageError,
				getCapability,
				getOptionalCapability,
				registerBeforeSave: hook => {
					if (typeof hook !== "function") throw new TypeError("Editor before-save hook must be a function");
					this.beforeSaveHooks.push(hook);
					return toDisposable(() => {
						const index = this.beforeSaveHooks.indexOf(hook);
						if (index >= 0) this.beforeSaveHooks.splice(index, 1);
					});
				},
				register: value => this._register(value),
			};
			for (const contribution of selectedContributions) contribution.install?.(installContext);
			const runtimeContributions = selectedContributions.flatMap(contribution => contribution.runtime ? [{
				id: contribution.id,
				descriptor: contribution.runtime.descriptor,
				instantiation: contribution.runtime.instantiation,
			}] : []);
			if (runtimeContributions.length > 0) {
				this.codeEditor.contributions.add(installContext, runtimeContributions);
			}
		} catch (error) {
			this.dispose();
			throw error;
		}
	}

	public registerEditorLifetime<T extends IDisposable>(value: T): T {
		return this._register(value);
	}

	layout(dimension: IDimension): void { this.codeEditor.layout(dimension); }
	announceAccessibilityStatus(message: string): void { this.codeEditor.announceAccessibilityStatus(message); }
	focus(): void { this.codeEditor.focus(); }
	getValue(): string { return this.codeEditor.getValue(); }
	setValue(value: string): void { this.codeEditor.setValue(value); }
	revealRange(range: TextRange): void { this.codeEditor.revealRange(range); }
	getViewState(): EditorTextViewState { return this.codeEditor.saveViewState(); }
	restoreViewState(state: EditorTextViewState): void { this.codeEditor.restoreViewState(state); }
	addContentWidget(widget: IContentWidget): void { this.codeEditor.addContentWidget(widget); }
	layoutContentWidget(widget: IContentWidget): void { this.codeEditor.layoutContentWidget(widget); }
	removeContentWidget(widget: IContentWidget): void { this.codeEditor.removeContentWidget(widget); }
	addOverlayWidget(widget: IOverlayWidget): void { this.codeEditor.addOverlayWidget(widget); }
	layoutOverlayWidget(widget: IOverlayWidget): void { this.codeEditor.layoutOverlayWidget(widget); }
	removeOverlayWidget(widget: IOverlayWidget): void { this.codeEditor.removeOverlayWidget(widget); }
	changeViewZones(callback: (accessor: IViewZoneChangeAccessor) => void): void { this.codeEditor.changeViewZones(callback); }
	async prepareSave(): Promise<void> {
		for (const hook of [...this.beforeSaveHooks]) await hook();
	}
}

function isViewPosition(value: unknown): value is EditorTextViewPositionState {
	if (!value || typeof value !== "object") return false;
	const position = value as Partial<EditorTextViewPositionState>;
	return isSafeInteger(position.lineIndex) && position.lineIndex! >= 0 && isSafeInteger(position.columnIndex) && position.columnIndex! >= 0;
}

function isViewScrollPosition(value: unknown): value is EditorTextViewState["scrollPosition"] {
	if (!value || typeof value !== "object") return false;
	const position = value as Partial<EditorTextViewState["scrollPosition"]>;
	return isFiniteNumber(position.left) && position.left! >= 0 && isFiniteNumber(position.top) && position.top! >= 0;
}

function validateOptions(options: EditorBrowserOptions): void {
	if (!options || typeof options !== "object" || !options.container || !options.model) {
		throw new TypeError("Editor browser requires a container and text model");
	}
	if (options.input?.readOnly !== undefined && typeof options.input.readOnly !== "boolean") {
		throw new TypeError("Editor input read-only mode must be boolean");
	}
	if (options.onLanguageError !== undefined && typeof options.onLanguageError !== "function") {
		throw new TypeError("Editor language error handler must be a function");
	}
	if (options.insertFinalNewLine !== undefined && typeof options.insertFinalNewLine !== "boolean") {
		throw new TypeError("Editor final newline option must be boolean");
	}
	if (options.occurrencesHighlight !== undefined && options.occurrencesHighlight !== "off" && options.occurrencesHighlight !== "singleFile" && options.occurrencesHighlight !== "multiFile") {
		throw new TypeError("Editor occurrences highlight option is invalid");
	}
	if (options.occurrencesHighlightDelay !== undefined && (!Number.isSafeInteger(options.occurrencesHighlightDelay) || options.occurrencesHighlightDelay < 0 || options.occurrencesHighlightDelay > 2_000)) {
		throw new RangeError("Editor occurrences highlight delay must be an integer between 0 and 2000");
	}
	if (options.selectionHighlightMaxLength !== undefined && (!Number.isSafeInteger(options.selectionHighlightMaxLength) || options.selectionHighlightMaxLength < 0)) {
		throw new RangeError("Editor selection highlight maximum length must be a non-negative integer");
	}
	if (options.matchBrackets !== undefined && options.matchBrackets !== "never" && options.matchBrackets !== "near" && options.matchBrackets !== "always") {
		throw new TypeError("Editor bracket matching option is invalid");
	}
	if (options.colorDecoratorsActivatedOn !== undefined && options.colorDecoratorsActivatedOn !== 'clickAndHover' && options.colorDecoratorsActivatedOn !== 'click' && options.colorDecoratorsActivatedOn !== 'hover') {
		throw new TypeError('Editor color decorator activation is invalid');
	}
	if (options.defaultColorDecorators !== undefined && options.defaultColorDecorators !== 'auto' && options.defaultColorDecorators !== 'always' && options.defaultColorDecorators !== 'never') {
		throw new TypeError('Editor default color decorators option is invalid');
	}
	if (options.colorDecoratorsLimit !== undefined && (!Number.isSafeInteger(options.colorDecoratorsLimit) || options.colorDecoratorsLimit < 0)) {
		throw new RangeError('Editor color decorator limit must be a non-negative integer');
	}
	if (options.suggestions !== undefined && !isCompletionsEnablement(options.suggestions)) {
		throw new TypeError("Editor suggestions option must be boolean or a language enablement map");
	}
	if (options.inlineCompletions !== undefined && !isCompletionsEnablement(options.inlineCompletions)) {
		throw new TypeError("Editor inline completions option must be boolean or a language enablement map");
	}
	if (options.renderWhitespace !== undefined && !['none', 'boundary', 'selection', 'trailing', 'all'].includes(options.renderWhitespace)) {
		throw new TypeError('Editor whitespace rendering option is invalid');
	}
	if (options.lineNumbers !== undefined && typeof options.lineNumbers !== 'function' && !['on', 'off', 'relative', 'interval'].includes(options.lineNumbers)) {
		throw new TypeError('Editor line numbers option is invalid');
	}
	if (options.renderLineHighlight !== undefined && !['none', 'gutter', 'line', 'all'].includes(options.renderLineHighlight)) {
		throw new TypeError('Editor line highlight option is invalid');
	}
	if (options.cursorStyle !== undefined && !['line', 'block', 'underline', 'line-thin', 'block-outline', 'underline-thin'].includes(options.cursorStyle)) {
		throw new TypeError('Editor cursor style option is invalid');
	}
	if (options.cursorBlinking !== undefined && !['blink', 'smooth', 'phase', 'expand', 'solid'].includes(options.cursorBlinking)) {
		throw new TypeError('Editor cursor blinking option is invalid');
	}
	for (const [name, value] of [['cursor width', options.cursorWidth], ['cursor height', options.cursorHeight]] as const) {
		if (value !== undefined && (!isSafeInteger(value) || value < 0)) throw new RangeError(`Editor ${name} must be a non-negative safe integer`);
	}
	for (const [name, value] of [
		["glyph margin", options.glyphMargin],
		["symbol icons", options.showSymbolIcons],
		['line highlight focus', options.renderLineHighlightOnlyWhenFocus],
		["bracket pair colorization", options.bracketPairColorization],
		["sticky scroll", options.stickyScroll],
		["parameter hints", options.parameterHints],
		["inlay hints", options.inlayHints],
		["CodeLens", options.codeLens],
		['color decorators', options.colorDecorators],
		["format on save", options.formatOnSave],
		["selection highlight", options.selectionHighlight],
		["multiline selection highlight", options.selectionHighlightMultiline],
		['content widget overflow', options.allowOverflow],
		['fixed content widget overflow', options.fixedOverflowWidgets],
	] as const) {
		if (value !== undefined && typeof value !== "boolean") throw new TypeError(`Editor ${name} option must be boolean`);
	}
}

function editorLabel(input: EditorResourceInput): string {
	if (input.label?.trim()) return input.label;
	const path = decodeURIComponent(input.resource.path);
	return path.slice(path.lastIndexOf("/") + 1) || "Text editor";
}

function reportLanguageError(error: unknown): void {
	console.error("Editor language request failed", error);
}
