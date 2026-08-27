import { type IDimension } from "../../base/browser/geometry.js";
import { isNonEmptyArray } from "../../base/common/arrays.js";
import { type Event } from "../../base/common/event.js";
import { Disposable, type IDisposable, toDisposable } from "../../base/common/lifecycle.js";
import { isFiniteNumber, isSafeInteger } from "../../base/common/numbers.js";
import { type ISyntaxApi } from "../../platform/syntax/common/syntaxApi.js";
import { type EditorResourceInput } from "../common/editorResource.js";
import { EditorSelectionController } from "../common/cursor/editorSelectionController.js";
import { TextSelection, TextSelectionSet } from "../common/core/selection.js";
import { TextPosition, type TextRange } from "../common/core/text.js";
import { type LanguageCompletionWorkerFactory } from "../common/languages/completion/languageCompletionService.js";
import { type SyntaxWorkerFactory } from "../common/languages/syntax/syntaxService.js";
import { LanguageFeaturesService, type ILanguageFeaturesService } from "../common/services/languageService.js";
import { type TextModelReference } from "../common/services/textModelService.js";
import { type EditorIndentationOptions } from "../common/editorIndentation.js";
import { type EditorActiveLineHighlight, type EditorLanguageEditingAdapter, type EditorMinimap, type EditorRuler, type EditorTextDirection, type EditorView, type EditorViewport, type EditorViewportPresentation } from "./view.js";
import { CodeEditorWidget, type CodeEditorViewPositionState, type CodeEditorViewSelectionState, type CodeEditorViewState } from "./widget/codeEditor/codeEditorWidget.js";
import { type EditorHitTarget } from "../common/viewModel/pointerHitTest.js";
import { type EditorLineWrapping, type WrappingIndent } from "../common/config/editorOptions.js";
import { type LanguageLocation } from "../contrib/gotoSymbol/common/languageNavigation.js";
import { type LanguageWorkspaceEdit } from "../common/languages/languageWorkspaceEdit.js";
import { type ILanguageDiagnosticsService } from "../common/services/languageDiagnosticsService.js";
import { combineEditorLineGutterDecorations, type EditorLineGutterDecoration } from "./viewparts/margin/lineGutterDecoration.js";
import { type DecorationSource, type OwnedDecorationSource } from "./viewparts/decorations/decorationPresentation.js";
import { type IDiffApi } from "../../platform/diff/common/diffApi.js";
import { type IInstantiationService } from "../../platform/instantiation/common/instantiation.js";
import { type IAccessibilityService } from "../../platform/accessibility/common/accessibility.js";
import { TabFocus } from "./config/tabFocus.js";
import { resolveEditorConfiguration } from "./config/editorConfiguration.js";
import { getEditorContributions, type EditorCapability, type TextEditorContributionContext } from "./editorExtensions.js";
import { type BracketColorizationSource, type SemanticTokenSource } from "./viewparts/semanticTokens/semanticTokenPresentation.js";
import { type EditorLineVisibilitySource } from "../common/viewModel/viewModelLines.js";
import { type LanguageLexicalContextSource } from "../common/languages/languageLexicalContext.js";

export interface EditorContextMenuRequest {
	readonly position: TextPosition;
	readonly target: EditorHitTarget | undefined;
	readonly clientX: number;
	readonly clientY: number;
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
	/** Optional Rust-backed syntax facts used for parser-grade fold ranges. */
	readonly syntaxApi?: ISyntaxApi;
	/** Optional Rust-backed line diff API exposed to editor-local contributions. */
	readonly diffApi?: IDiffApi;
	/** Window-scoped constructor service for runtime editor contributions. */
	readonly instantiationService?: IInstantiationService;
	/** Optional accessibility policy used by native screen-reader content. */
	readonly accessibilityService?: IAccessibilityService;
	/** Chooses line-structured content for native screen-reader projection. */
	readonly renderRichScreenReaderContent?: boolean;
	/** Controls how many logical lines one native screen-reader page exposes. */
	readonly accessibilityPageSize?: number;
	/** Optional host service that synchronizes open models and supplies push diagnostics. */
	readonly languageDiagnosticsService?: ILanguageDiagnosticsService;
	readonly modelReference: TextModelReference;
	readonly syntaxWorkerFactory?: SyntaxWorkerFactory;
	readonly completionWorkerFactory?: LanguageCompletionWorkerFactory;
	readonly languageSupport?: IDisposable;
	readonly onDidChangeLanguageSupport?: Event<void>;
	readonly whenLanguageSupportReady?: () => Promise<unknown>;
	readonly onLanguageError?: (error: unknown) => void;
	readonly onSave?: () => Promise<void | boolean>;
	readonly onRevert?: () => Promise<void>;
	readonly indentation?: EditorIndentationOptions;
	readonly lineWrapping?: EditorLineWrapping;
	readonly wrappingIndent?: WrappingIndent;
	readonly fontFamily?: string;
	readonly fontSize?: number;
	readonly lineHeight?: number;
	readonly fontLigatures?: boolean;
	readonly minimap?: EditorMinimap;
	readonly activeLineHighlight?: EditorActiveLineHighlight;
	readonly showLineNumbers?: boolean;
	readonly rulers?: readonly EditorRuler[];
	readonly showIndentationGuides?: boolean;
	readonly bracketPairColorization?: boolean;
	readonly stickyScroll?: boolean;
	readonly suggestions?: boolean;
	readonly inlineCompletions?: boolean;
	readonly parameterHints?: boolean;
	readonly inlayHints?: boolean;
	readonly codeLens?: boolean;
	readonly formatOnSave?: boolean;
	readonly find?: EditorFindOptions;
	/** Applies a single LF at the save boundary when the document has content and no final LF. */
	readonly insertFinalNewLine?: boolean;
	/** Browser paragraph direction for this editor browser's DOM projection. */
	readonly textDirection?: EditorTextDirection;
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
	readonly lineGutterDecorations?: readonly EditorLineGutterDecoration[];
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
	readonly isDirty: boolean;
	readonly hasExternalChange: boolean;
	save(): Promise<void>;
	revert(): Promise<void>;
}

/** Browser composition root for the line editor. */
export class EditorBrowser extends Disposable implements IEditorBrowser {
	private readonly modelReference: TextModelReference;
	private readonly onSave: (() => Promise<void | boolean>) | undefined;
	private readonly onRevert: (() => Promise<void>) | undefined;
	private readonly beforeSaveHooks: Array<() => void | Promise<void>> = [];
	readonly onDidChange: Event<void>;
	readonly codeEditor: CodeEditorWidget;
	readonly viewport: EditorViewport;
	readonly selections: EditorSelectionController;
	readonly view: EditorView;

	constructor(options: EditorBrowserOptions) {
		super();
		try {
			validateOptions(options);
			const configuration = resolveEditorConfiguration(options);
			const tabFocus = options.tabFocus ?? this._register(new TabFocus());
			const languageId = options.languageId;
			const onLanguageError = options.onLanguageError ?? reportLanguageError;
			this.onSave = options.onSave;
			this.onRevert = options.onRevert;
			if (options.languageSupport) this._register(options.languageSupport);
			const modelReference = this.modelReference = this._register(options.modelReference);
			const model = modelReference.model;
			this.onDidChange = listener => model.onDidChange(() => listener());
			const languageFeaturesService = options.languageFeaturesService ?? this._register(new LanguageFeaturesService());
			const configurations = languageFeaturesService.configurations;
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
			const lineGutterDecorations: EditorLineGutterDecoration[] = [...(options.lineGutterDecorations ?? [])];
			let lineProjection: { readonly visibilitySource: EditorLineVisibilitySource; readonly gutterDecoration?: EditorLineGutterDecoration } | undefined;
			let semanticTokenSource: SemanticTokenSource | undefined;
			let bracketColorizationSource: BracketColorizationSource | undefined;
			let languageLexicalContext: LanguageLexicalContextSource | undefined;
			let languageEditing: EditorLanguageEditingAdapter | undefined;
			const selectedContributions = getEditorContributions();
			for (const contribution of selectedContributions) {
				contribution.configure?.({
					kind: "text",
					options,
					model,
					languageId,
					languageFeaturesService,
					configurations,
					selections: this.selections,
					tabFocus,
					onLanguageError,
					getCapability,
					getOptionalCapability,
					provideCapability,
					addDecorationSource: source => decorationSources.push(source),
					addLineGutterDecoration: decoration => lineGutterDecorations.push(decoration),
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
					setLanguageEditing: adapter => {
						if (languageEditing) throw new Error("Text editor language editing is already configured");
						languageEditing = adapter;
					},
					register: value => this._register(value),
				});
			}
			const ariaLabel = editorLabel(options.input);
			this.codeEditor = this._register(new CodeEditorWidget({
				container: options.container,
				model,
				selectionController: this.selections,
				lineHeight: configuration.lineHeight,
				ariaLabel,
				ownerId: options.ownerId,
				instantiationService: options.instantiationService,
				onContributionError: onLanguageError,
				viewport: {
					lineVisibilitySource: lineProjection?.visibilitySource,
					lineGutterDecoration: combineEditorLineGutterDecorations([...(lineProjection?.gutterDecoration ? [lineProjection.gutterDecoration] : []), ...lineGutterDecorations]),
					decorationSources,
					semanticTokenSource,
					bracketColorizationSource,
					lineWrapping: options.lineWrapping,
					wrappingIndent: options.wrappingIndent,
					fontFamily: configuration.fontFamily,
					fontSize: configuration.fontSize,
					fontLigatures: configuration.fontLigatures,
					showLineNumbers: options.showLineNumbers,
					rulers: options.rulers,
					showIndentationGuides: options.showIndentationGuides,
					minimap: options.minimap,
					activeLineHighlight: options.activeLineHighlight,
					textDirection: options.textDirection,
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
			this.viewport = this.codeEditor.viewport;
			this.view = this.codeEditor.view;
			this._register(modelReference.onDidChangeExternalChange(() => {
				if (modelReference.hasExternalChange) this.codeEditor.announceAccessibilityStatus("File changed on disk. Local edits are preserved.");
			}));
			const installContext: TextEditorContributionContext = {
				kind: "text",
				options,
				model,
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
				if (!options.instantiationService) throw new Error("Runtime editor contributions require an instantiation service");
				this.codeEditor.contributions.add(installContext, runtimeContributions);
			}
		} catch (error) {
			this.dispose();
			throw error;
		}
	}

	layout(dimension: IDimension): void { this.codeEditor.layout(dimension); }
	announceAccessibilityStatus(message: string): void { this.codeEditor.announceAccessibilityStatus(message); }
	focus(): void { this.codeEditor.focus(); }
	getValue(): string { return this.codeEditor.getValue(); }
	setValue(value: string): void { this.codeEditor.setValue(value); }
	revealRange(range: TextRange): void { this.codeEditor.revealRange(range); }
	getViewState(): EditorTextViewState { return this.codeEditor.saveViewState(); }
	restoreViewState(state: EditorTextViewState): void { this.codeEditor.restoreViewState(state); }
	get isDirty(): boolean { return this.modelReference.isDirty; }
	get hasExternalChange(): boolean { return this.modelReference.hasExternalChange; }
	async save(): Promise<void> {
		for (const hook of [...this.beforeSaveHooks]) await hook();
		await this.onSave?.();
	}
	async revert(): Promise<void> { await this.onRevert?.(); }
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
	if (!options || typeof options !== "object" || !options.container || !options.modelReference) {
		throw new TypeError("Editor browser requires a container and model reference");
	}
	if (options.input?.readOnly !== undefined && typeof options.input.readOnly !== "boolean") {
		throw new TypeError("Editor input read-only mode must be boolean");
	}
	if (options.whenLanguageSupportReady !== undefined && typeof options.whenLanguageSupportReady !== "function") {
		throw new TypeError("Editor language readiness must be a function");
	}
	if (options.onLanguageError !== undefined && typeof options.onLanguageError !== "function") {
		throw new TypeError("Editor language error handler must be a function");
	}
	if (options.onSave !== undefined && typeof options.onSave !== "function") {
		throw new TypeError("Editor save must be a function");
	}
	if (options.onRevert !== undefined && typeof options.onRevert !== "function") {
		throw new TypeError("Editor revert must be a function");
	}
	if (options.insertFinalNewLine !== undefined && typeof options.insertFinalNewLine !== "boolean") {
		throw new TypeError("Editor final newline option must be boolean");
	}
	for (const [name, value] of [
		["line numbers", options.showLineNumbers],
		["indentation guides", options.showIndentationGuides],
		["bracket pair colorization", options.bracketPairColorization],
		["sticky scroll", options.stickyScroll],
		["suggestions", options.suggestions],
		["inline completions", options.inlineCompletions],
		["parameter hints", options.parameterHints],
		["inlay hints", options.inlayHints],
		["CodeLens", options.codeLens],
		["format on save", options.formatOnSave],
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
