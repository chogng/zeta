import { type IDimension } from "../../base/browser/geometry.js";
import { type Event } from "../../base/common/event.js";
import { DisposableOwner, toDisposable } from "../../base/common/lifecycle.js";
import { type EditorResourceInput } from "../common/editorResource.js";
import { type TextModelReference } from "../common/services/textModelService.js";
import { CodeEditorWidget } from "./widget/codeEditor/codeEditorWidget.js";
import { type EditorViewport } from "./view/editorViewport.js";
import { type EditorLanguageEditingAdapter, type EditorView } from "./view.js";
import { LanguageFeaturesService } from "../common/services/languageService.js";
import { EditorSelectionController } from "../common/cursor/editorSelectionController.js";
import { TextSelection, TextSelectionSet } from "../common/core/selection.js";
import { TextPosition, type TextRange } from "../common/core/text.js";
import { type EditorBrowserOptions, type EditorTextViewState, type IEditorBrowserRuntime } from "./editorBrowser.js";
import { getEditorContributions, type EditorCapability, EditorContributionInstantiation, type EditorContribution, type TextEditorContributionContext } from "./editorExtensions.js";
import { type DecorationSource } from "./viewparts/decorations/decorationPresentation.js";
import { combineEditorLineGutterDecorations, type EditorLineGutterDecoration } from "./viewparts/margin/lineGutterDecoration.js";
import { type BracketColorizationSource, type SemanticTokenSource } from "./viewparts/semanticTokens/semanticTokenPresentation.js";
import { type EditorLineVisibilitySource } from "../common/viewModel/modelLineProjection.js";
import { type LanguageLexicalContextSource } from "../common/languages/languageLexicalContext.js";
import { runWhenWindowIdle, scheduleAtNextAnimationFrame } from "../../base/browser/scheduler.js";
import { getWindow } from "../../base/browser/window.js";
import { resolveEditorConfiguration } from "./config/editorConfiguration.js";
import { TabFocus } from "./config/tabFocus.js";

/**
 * Owns one code-editor runtime after the browser composition root has selected
 * the statically loaded contribution set.
 */
export class EditorBrowserRuntime extends DisposableOwner implements IEditorBrowserRuntime {
	readonly onDidChange: Event<void>;
	readonly codeEditor: CodeEditorWidget;
	readonly viewport: EditorViewport;
	readonly selections: EditorSelectionController;
	readonly view: EditorView;
	private readonly languageId: string;
	private readonly onLanguageError: (error: unknown) => void;
	private readonly modelReference: TextModelReference;
	private readonly onSave: (() => Promise<void | boolean>) | undefined;
	private readonly onRevert: (() => Promise<void>) | undefined;
	private readonly beforeSaveHooks: Array<() => void | Promise<void>> = [];

	constructor(options: EditorBrowserOptions) {
		super();
		try {
			validateOptions(options);
			const configuration = resolveEditorConfiguration(options);
			const tabFocus = options.tabFocus ?? this.own(new TabFocus());
			this.languageId = options.languageId;
			this.onLanguageError = options.onLanguageError ?? reportLanguageError;
			this.onSave = options.onSave;
			this.onRevert = options.onRevert;
			if (options.languageSupport) this.own(options.languageSupport);
			const modelReference = this.modelReference = this.own(options.modelReference);
			const model = modelReference.model;
			this.onDidChange = listener => model.onDidChange(() => listener());
			const languageFeaturesService = options.languageFeaturesService ?? this.own(new LanguageFeaturesService());
			const configurations = languageFeaturesService.configurations;
			this.selections = this.own(new EditorSelectionController(
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
			for (const source of options.decorationSources ?? []) decorationSources.push(this.own(source));
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
					languageId: this.languageId,
					languageFeaturesService,
					configurations,
					selections: this.selections,
					tabFocus,
					onLanguageError: this.onLanguageError,
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
					own: value => this.own(value),
				});
			}
			const ariaLabel = editorLabel(options.input);
			const lineHeight = configuration.lineHeight;

			this.codeEditor = this.own(new CodeEditorWidget({
				container: options.container,
				model,
				lineHeight,
				selectionController: this.selections,
				ownerId: options.ownerId,
				ariaLabel,
				placeholder: options.placeholder,
				instantiationService: options.instantiationService,
				onContributionError: this.onLanguageError,
				viewport: {
					lineVisibilitySource: lineProjection?.visibilitySource,
					lineGutterDecoration: combineEditorLineGutterDecorations([...lineGutterDecorations, ...(lineProjection?.gutterDecoration ? [lineProjection.gutterDecoration] : [])]),
					decorationSources,
					semanticTokenSource,
					bracketColorizationSource,
					lineWrapping: options.lineWrapping,
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
				wordPattern: () => configurations.getLanguageConfiguration(this.languageId).wordPattern,
				keyboardNavigation: {
					wordPattern: () => configurations.getLanguageConfiguration(this.languageId).wordPattern,
				},
				mouseHandler: {
					wordPattern: () => configurations.getLanguageConfiguration(this.languageId).wordPattern,
				},
			}));
			this.viewport = this.codeEditor.viewport;
			this.view = this.codeEditor.view;
			this.own(modelReference.onDidChangeExternalChange(() => {
				if (modelReference.hasExternalChange) {
					this.viewport.announceAccessibilityStatus("File changed on disk. Local edits are preserved.");
				}
			}));
			const installContext: TextEditorContributionContext = {
				kind: "text",
				options,
				model,
				languageId: this.languageId,
				languageFeaturesService,
				configurations,
				view: this.view,
				viewport: this.viewport,
				selections: this.selections,
				tabFocus,
				onLanguageError: this.onLanguageError,
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
				own: value => this.own(value),
			};
			for (const contribution of selectedContributions) contribution.install?.(installContext);
			this.installRuntimeContributions(selectedContributions, installContext, options);
		} catch (error) {
			this.dispose();
			throw error;
		}
	}

	layout(dimension: IDimension): void {
		this.codeEditor.layout(dimension);
	}

	announceAccessibilityStatus(message: string): void {
		this.viewport.announceAccessibilityStatus(message);
	}

	focus(): void {
		this.codeEditor.focus();
	}

	getValue(): string {
		return this.viewport.textModel.getText();
	}

	setValue(value: string): void {
		if (this.getValue() === value) return;
		this.modelReference.model.reset(value);
	}

	revealRange(range: TextRange): void {
		this.viewport.textModel.offsetAt(range.start);
		this.viewport.textModel.offsetAt(range.end);
		this.selections.setSelections(TextSelectionSet.single(TextSelection.from(range.start, range.end)));
		this.viewport.revealPosition(range.start);
	}

	getViewState(): EditorTextViewState {
		return Object.freeze({
			selections: Object.freeze(this.selections.selections.selections.map(selection => Object.freeze({
				anchor: Object.freeze({ lineIndex: selection.anchor.lineIndex, columnIndex: selection.anchor.columnIndex }),
				active: Object.freeze({ lineIndex: selection.active.lineIndex, columnIndex: selection.active.columnIndex }),
			}))),
			primarySelectionIndex: this.selections.selections.primaryIndex,
			scrollPosition: Object.freeze({ ...this.viewport.currentLayout.scrollPosition }),
		});
	}

	restoreViewState(state: EditorTextViewState): void {
		const selections = state.selections.map(selection => {
			const anchor = TextPosition.at(selection.anchor.lineIndex, selection.anchor.columnIndex);
			const active = TextPosition.at(selection.active.lineIndex, selection.active.columnIndex);
			this.modelReference.model.offsetAt(anchor);
			this.modelReference.model.offsetAt(active);
			return TextSelection.from(anchor, active);
		});
		this.selections.setSelections(TextSelectionSet.withPrimary(selections, state.primarySelectionIndex));
		this.viewport.scrollTo(state.scrollPosition);
	}

	get isDirty(): boolean {
		return this.modelReference.isDirty;
	}

	get hasExternalChange(): boolean {
		return this.modelReference.hasExternalChange;
	}

	async save(): Promise<void> {
		for (const hook of [...this.beforeSaveHooks]) await hook();
		await this.onSave?.();
	}

	async revert(): Promise<void> {
		await this.onRevert?.();
	}

	private installRuntimeContributions(contributions: readonly EditorContribution[], context: TextEditorContributionContext, options: EditorBrowserOptions): void {
		const runtimeContributions = contributions.filter(contribution => contribution.runtime !== undefined);
		if (runtimeContributions.length === 0) return;
		const instantiationService = options.instantiationService;
		if (!instantiationService) throw new Error("Runtime editor contributions require an instantiation service");
		const targetWindow = getWindow(this.viewport.element);
		for (const contribution of runtimeContributions) {
			const instantiate = (): void => {
				if (this.isDisposed || !contribution.runtime) return;
				try {
					this.own(instantiationService.createInstance(contribution.runtime.descriptor, context));
				} catch (error) {
					if (contribution.runtime.instantiation === EditorContributionInstantiation.Eager) throw error;
					this.onLanguageError(error);
				}
			};
			switch (contribution.runtime!.instantiation) {
				case EditorContributionInstantiation.Eager:
					instantiate();
					break;
				case EditorContributionInstantiation.AfterFirstRender:
					this.own(scheduleAtNextAnimationFrame(targetWindow, instantiate));
					break;
				case EditorContributionInstantiation.Eventually:
					this.own(runWhenWindowIdle(targetWindow, instantiate, { timeoutMs: 5_000 }));
					break;
			}
		}
	}
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
