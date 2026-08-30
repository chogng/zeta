import { isHTMLElement } from "../../../../base/browser/dom.js";
import { getClientArea } from "../../../../base/browser/dom.js";
import { isNonEmptyArray } from "../../../../base/common/arrays.js";
import { Emitter, type Event } from "../../../../base/common/event.js";
import { Disposable, type IDisposable } from "../../../../base/common/lifecycle.js";
import { isFiniteNumber, isSafeInteger } from "../../../../base/common/numbers.js";
import { CursorsController } from "../../../common/cursor/cursor.js";
import { type IDimension } from '../../../common/core/2d/dimension.js';
import { Selection } from "../../../common/core/selection.js";
import { SelectionSet } from "../../../common/cursor/selectionSet.js";
import { Position } from "../../../common/core/position.js";
import { type Range } from "../../../common/core/range.js";
import { type TextModel } from "../../../common/model/textModel.js";
import { type EditorScrollPosition } from "../../../common/viewModel/editorViewportContracts.js";
import type { IContentWidget, IOverlayWidget, IViewZoneChangeAccessor } from '../../editorBrowser.js';
import { EditorView, type EditorViewOptions, type EditorViewViewportOptions } from '../../editorView.js';
import { type View } from "../../view.js";
import { KeyboardNavigationController } from "../../view/viewController.js";
import { EditorPointerSelectionHandler } from "../../controller/mouseHandler.js";
import { ServiceContainer, type IInstantiationService } from "../../../../platform/instantiation/common/instantiation.js";
import { WidgetContributionCollection, type CodeEditorContribution, type CodeEditorContributionDescription } from "./codeEditorContributions.js";
import { observableCodeEditor } from '../../observableCodeEditor.js';
import { type IEditorConstructionOptions } from '../../config/editorConfiguration.js';
import { migrateOptions } from '../../config/migrateOptions.js';
import { resolveEditorGeometryConfiguration } from '../../config/resolvedEditorGeometryConfiguration.js';
import { TabFocus } from '../../config/tabFocus.js';
import { type EditorResourceInput } from '../../editorInput.js';
import { getTextEditorCapabilityContributions, type EditorCapability, type EditorCommandEvent, type TextEditorContributionContext } from '../../editorExtensions.js';
import { VersionedEditorWorkerClient, type VersionedEditorWorkerFactory } from '../../services/editorWorkerService.js';
import { EditorWorkerRequestExecutor } from '../../../common/services/editorWorkerRequestExecutor.js';
import { createBuiltinLanguageConfigurationService } from '../../../common/languages/languageBuiltinConfigurations.js';
import { type ILanguageConfigurationService } from '../../../common/languages/languageConfigurationRegistry.js';
import type { ILanguageFeaturesService } from '../../../common/services/languageFeatures.js';
import { LanguageFeaturesService } from '../../../common/services/languageFeaturesService.js';
import { ResolvedSemanticTokensService } from '../../../common/services/resolvedSemanticTokensService.js';
import { LanguageEditingAdapter } from '../../view/viewController.js';
import { resolveEditorIndentationOptions, type EditorIndentationOptions } from '../../../common/core/misc/indentation.js';
import { EditorOptions, type EditorLineWrapping, WrappingIndent } from '../../../common/config/editorOptions.js';
import { type LanguageCompletionWorkerFactory } from '../../../common/languages/completion/languageCompletionService.js';
import { type ILanguageDiagnosticsService } from '../../../common/services/languageDiagnosticsService.js';
import { isCompletionsEnablement, type CompletionsEnablement } from '../../../common/services/ownedCompletionsEnablement.js';
import { type LanguageLocation } from '../../../contrib/gotoSymbol/common/languageNavigation.js';
import { type LanguageWorkspaceEdit } from '../../../common/languages/languageWorkspaceEdit.js';
import { type DecorationSource, type OwnedDecorationSource } from '../../viewParts/decorations/decorations.js';
import { type EditorLineVisibilitySource } from '../../../common/viewModel/viewModelLines.js';
import { type LanguageLexicalContextSource } from '../../../common/languages/languageLexicalContext.js';
import { type BracketColorizationSource, type SemanticTokenSource } from '../../viewParts/viewLines/viewLine.js';
import { type EditorTextDirection, type EditorViewportPresentation } from '../../view.js';
import { type EditorHitTarget } from '../../../common/viewModel/pointerHitTest.js';
import { type IAccessibilityService } from '../../../../platform/accessibility/common/accessibility.js';
import { type IWidgetCodeEditorRegistry } from '../../services/codeEditorService.js';

export type CodeEditorWidgetViewportOptions = EditorViewViewportOptions;

export interface EditorContextMenuRequest {
	readonly position: Position;
	readonly target: EditorHitTarget | undefined;
	readonly clientX: number;
	readonly clientY: number;
}

export interface EditorSectionHeaderOptions {
	readonly showRegionSectionHeaders?: boolean;
	readonly showMarkSectionHeaders?: boolean;
	readonly markSectionHeaderRegex?: string;
}

/** Internal services and host callbacks used while constructing one code editor widget. */
export interface ICodeEditorWidgetOptions {
	readonly container: HTMLElement;
	readonly input: EditorResourceInput;
	readonly languageId: string;
	readonly model: TextModel;
	readonly ownerId?: string;
	readonly tabFocus?: TabFocus;
	readonly languageFeaturesService?: ILanguageFeaturesService;
	readonly languageConfigurationService?: ILanguageConfigurationService;
	readonly instantiationService?: IInstantiationService;
	readonly codeEditorService?: IWidgetCodeEditorRegistry;
	readonly accessibilityService?: IAccessibilityService;
	readonly editorWorkerFactory?: VersionedEditorWorkerFactory;
	readonly completionWorkerFactory?: LanguageCompletionWorkerFactory;
	readonly languageDiagnosticsService?: ILanguageDiagnosticsService;
	readonly onLanguageError?: (error: unknown) => void;
	readonly onOpenLink?: (target: string) => void | Promise<void>;
	readonly onShowContextMenu?: (request: EditorContextMenuRequest) => void | Promise<void>;
	readonly onExecuteEditorCommand?: (id: string, args: readonly unknown[] | undefined) => void | Promise<void>;
	readonly onOpenLocation?: (location: LanguageLocation) => void | Promise<void>;
	readonly onApplyWorkspaceEdit?: (edit: LanguageWorkspaceEdit) => void | Promise<void>;
	readonly decorationSources?: readonly OwnedDecorationSource[];
	readonly registerBeforeSave?: (hook: () => void | Promise<void>) => IDisposable;
	readonly onContributionError?: (error: unknown) => void;
	readonly contributions?: readonly CodeEditorContributionDescription[];
	readonly sectionHeaders?: EditorSectionHeaderOptions | false;
	readonly suggestions?: CompletionsEnablement;
	readonly inlineCompletions?: CompletionsEnablement;
	readonly indentation?: EditorIndentationOptions;
	readonly lineWrapping?: EditorLineWrapping;
	readonly presentation?: EditorViewportPresentation;
	readonly textDirection?: EditorTextDirection;
	readonly showSymbolIcons?: boolean;
	readonly occurrencesHighlightDelay?: number;
	readonly selectionHighlightMaxLength?: number;
	readonly selectionHighlightMultiline?: boolean;
	readonly inlayHints?: boolean;
	readonly codeLens?: boolean;
	readonly stickyScroll?: boolean;
	readonly formatOnSave?: boolean;
	readonly showUnicodeHighlights?: boolean;
	readonly fontZoom?: { readonly initialScale?: number };
	readonly renderRichScreenReaderContent?: boolean;
	readonly placeholder?: string;
}

export type CodeEditorWidgetOptions = IEditorConstructionOptions & ICodeEditorWidgetOptions;

export interface CodeEditorViewPositionState {
	readonly lineIndex: number;
	readonly columnIndex: number;
}

export interface CodeEditorViewSelectionState {
	readonly anchor: CodeEditorViewPositionState;
	readonly active: CodeEditorViewPositionState;
}

export interface CodeEditorViewState {
	readonly selections: readonly CodeEditorViewSelectionState[];
	readonly primarySelectionIndex: number;
	readonly scrollPosition: EditorScrollPosition;
}

/**
 * Canonical browser editing surface for one Stanza text model and editor-local selection controller.
 *
 * Callers retain ownership of the model and selection controller. This component owns their DOM
 * projection plus native text input, keyboard navigation, and pointer selection. Optional
 * drop/paste behavior belongs to the host's contribution composition.
 */
export class CodeEditorWidget extends Disposable {
	readonly onDidChange: Event<void>;
	readonly selections: CursorsController;
	readonly ownerId: string;
	readonly view: EditorView;
	readonly viewport: View;
	readonly userInputEvents: EditorView['userInputEvents'];
	readonly contributions: WidgetContributionCollection;

	constructor(options: CodeEditorWidgetOptions) {
		super();
		try {
			validateOptions(options);
			migrateOptions(options);
			const geometry = resolveEditorGeometryConfiguration({
				fontFamily: options.fontFamily,
				fontSize: options.fontSize,
				lineHeight: options.lineHeight,
				fontLigatures: typeof options.fontLigatures === 'boolean' ? options.fontLigatures : undefined,
			});
			const tabFocus = options.tabFocus ?? this._register(new TabFocus());
			const onLanguageError = options.onLanguageError ?? options.onContributionError ?? reportLanguageError;
			const editorWorker = this._register(options.editorWorkerFactory
				? options.editorWorkerFactory(options.model)
				: new VersionedEditorWorkerClient(options.model, () => new EditorWorkerRequestExecutor()));
			this.onDidChange = listener => options.model.onDidChangeContent(() => listener());
			if (options.languageFeaturesService && !options.languageConfigurationService) {
				throw new TypeError('Editor language features require their language configuration service');
			}
			const languageConfigurationService = options.languageConfigurationService ?? this._register(createBuiltinLanguageConfigurationService());
			const languageFeaturesService = options.languageFeaturesService ?? this._register(new LanguageFeaturesService(languageConfigurationService));
			const resolvedSemanticTokensService = this._register(new ResolvedSemanticTokensService());
			this.selections = this._register(new CursorsController(
				options.model,
				SelectionSet.single(Selection.fromPositions(new Position(1, 1))),
				{ readOnly: options.input.readOnly },
			));
			const capabilities = new Map<string, unknown>();
			const commandEmitter = this._register(new Emitter<EditorCommandEvent>());
			const executeCommand = <T>(commandId: string, operation: () => T): T => executeEditorCommand(commandEmitter, commandId, operation);
			const getCapability = <T>(capability: EditorCapability<T>): T => {
				if (!capabilities.has(capability.id)) throw new ReferenceError(`Text editor capability '${capability.id}' is unavailable`);
				return capabilities.get(capability.id) as T;
			};
			const getOptionalCapability = <T>(capability: EditorCapability<T>): T | undefined => capabilities.get(capability.id) as T | undefined;
			const provideCapability = <T>(capability: EditorCapability<T>, value: T): void => {
				if (capabilities.has(capability.id)) throw new RangeError(`Text editor capability '${capability.id}' is already provided`);
				capabilities.set(capability.id, value);
			};
			const decorationSources: DecorationSource[] = [];
			for (const source of options.decorationSources ?? []) decorationSources.push(this._register(source));
			let lineProjection: { readonly visibilitySource: EditorLineVisibilitySource } | undefined;
			let semanticTokenSource: SemanticTokenSource | undefined;
			let bracketColorizationSource: BracketColorizationSource | undefined;
			let languageLexicalContext: LanguageLexicalContextSource | undefined;
			const selectedContributions = getTextEditorCapabilityContributions();
			for (const contribution of selectedContributions) {
				contribution.configure?.({
					kind: 'text',
					options,
					model: options.model,
					editorWorker,
					languageId: options.languageId,
					languageFeaturesService,
					resolvedSemanticTokensService,
					configurations: languageConfigurationService,
					tabFocus,
					onLanguageError,
					getCapability,
					getOptionalCapability,
					provideCapability,
					addDecorationSource: source => decorationSources.push(source),
					setLineProjection: projection => {
						if (lineProjection) throw new Error('Text editor line projection is already configured');
						lineProjection = projection;
					},
					setSemanticTokenSource: source => {
						if (semanticTokenSource) throw new Error('Text editor semantic-token source is already configured');
						semanticTokenSource = source;
					},
					setBracketColorizationSource: source => {
						if (bracketColorizationSource) throw new Error('Text editor bracket-colorization source is already configured');
						bracketColorizationSource = source;
					},
					setLanguageLexicalContext: source => {
						if (languageLexicalContext) throw new Error('Text editor lexical context is already configured');
						languageLexicalContext = source;
					},
					register: value => this._register(value),
				});
			}
			const languageEditing = this._register(new LanguageEditingAdapter(
				options.model,
				this.selections,
				options.languageId,
				languageConfigurationService,
				languageLexicalContext,
				options.indentation,
			));
			this.view = this._register(new EditorView({
				ownerId: options.ownerId,
				container: options.container,
				model: options.model,
				lineHeight: geometry.lineHeight,
				ariaLabel: options.ariaLabel ?? editorLabel(options.input),
				selectionController: this.selections,
				viewport: {
					cursorOptions: { readOnly: options.input.readOnly, stickyTabStops: options.stickyTabStops },
					languageId: options.languageId,
					languageConfigurationService,
					lineVisibilitySource: lineProjection?.visibilitySource,
					decorationSources,
					semanticTokenSource,
					bracketColorizationSource,
					lineWrapping: options.lineWrapping,
					wrappingIndent: resolveWrappingIndent(options.wrappingIndent),
					fontFamily: geometry.fontFamily,
					fontSize: geometry.fontSize,
					fontLigatures: geometry.fontLigatures,
					lineNumbers: options.lineNumbers,
					glyphMargin: options.glyphMargin,
					rulers: options.rulers?.map(ruler => typeof ruler === 'number'
						? { column: ruler }
						: { column: ruler.column, ...(ruler.color ? { color: ruler.color } : {}) }),
					guides: options.guides,
					minimap: options.minimap,
					renderLineHighlight: options.renderLineHighlight,
					renderLineHighlightOnlyWhenFocus: options.renderLineHighlightOnlyWhenFocus,
					textDirection: options.textDirection,
					experimentalGpuAcceleration: options.experimentalGpuAcceleration,
					renderWhitespace: options.renderWhitespace,
					mouseStyle: options.mouseStyle,
					cursorStyle: options.cursorStyle,
					overtypeCursorStyle: options.overtypeCursorStyle,
					cursorBlinking: options.cursorBlinking,
					cursorSmoothCaretAnimation: options.cursorSmoothCaretAnimation,
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
				wordPattern: () => languageConfigurationService.getLanguageConfiguration(options.languageId).getWordDefinition(),
			}));
			this.ownerId = this.view.ownerId;
			this.viewport = this.view.viewport;
			this.userInputEvents = this.view.userInputEvents;
			this._register(observableCodeEditor(this));
			this.contributions = this._register(new WidgetContributionCollection());
			const instantiationService = this._register(options.instantiationService?.createChild() ?? new ServiceContainer());
			this.contributions.initialize({
				editor: this,
				model: options.model,
				selectionController: this.selections,
				viewport: this.viewport,
				view: this.view,
				placeholder: options.placeholder,
			}, instantiationService, options.contributions, options.onContributionError);
			this._register(new KeyboardNavigationController(this.viewport, this.selections, this.userInputEvents, {
				wordPattern: () => languageConfigurationService.getLanguageConfiguration(options.languageId).getWordDefinition(),
				stickyTabStops: EditorOptions.stickyTabStops.validate(options.stickyTabStops) as boolean,
				tabSize: resolveEditorIndentationOptions(options.indentation).tabSize,
			}));
			this._register(new EditorPointerSelectionHandler(this.viewport, this.selections));
			if (options.codeEditorService) this._register(options.codeEditorService.addCodeEditor(this));
			const installContext: TextEditorContributionContext = {
				kind: 'text',
				options,
				model: options.model,
				editorWorker,
				languageId: options.languageId,
				languageFeaturesService,
				configurations: languageConfigurationService,
				view: this.view,
				viewport: this.viewport,
				viewModel: this.selections,
				tabFocus,
				onLanguageError,
				onDidExecuteCommand: commandEmitter.event,
				executeCommand,
				getCapability,
				getOptionalCapability,
				registerBeforeSave: options.registerBeforeSave,
				register: value => this._register(value),
			};
			for (const contribution of selectedContributions) contribution.install?.(installContext);
			const runtimeContributions = selectedContributions.flatMap(contribution => contribution.runtime ? [{
				id: contribution.id,
				descriptor: contribution.runtime.descriptor,
				instantiation: contribution.runtime.instantiation,
			}] : []);
			if (runtimeContributions.length > 0) this.contributions.add(installContext, runtimeContributions);
		} catch (error) {
			this.dispose();
			throw error;
		}
	}

	get element(): HTMLDivElement {
		return this.viewport.element;
	}

	layout(dimension: IDimension = getClientArea(this.element)): void {
		this.viewport.layout({ width: Math.max(0, dimension.width), height: Math.max(0, dimension.height) });
	}

	focus(): void {
		this.view.focus();
	}

	addContentWidget(widget: IContentWidget): void {
		this.viewport.addContentWidget(widget);
	}

	layoutContentWidget(widget: IContentWidget): void {
		this.viewport.layoutContentWidget(widget);
	}

	removeContentWidget(widget: IContentWidget): void {
		this.viewport.removeContentWidget(widget);
	}

	addOverlayWidget(widget: IOverlayWidget): void {
		this.viewport.addOverlayWidget(widget);
	}

	layoutOverlayWidget(widget: IOverlayWidget): void {
		this.viewport.layoutOverlayWidget(widget);
	}

	removeOverlayWidget(widget: IOverlayWidget): void {
		this.viewport.removeOverlayWidget(widget);
	}

	changeViewZones(callback: (accessor: IViewZoneChangeAccessor) => void): void {
		this.viewport.changeViewZones(callback);
	}

	announceAccessibilityStatus(message: string): void {
		this.viewport.announceAccessibilityStatus(message);
	}

	getValue(): string {
		return this.viewport.textModel.getText();
	}

	setValue(value: string): void {
		if (this.getValue() === value) return;
		this.viewport.textModel.reset(value);
	}

	revealRange(range: Range): void {
		this.viewport.textModel.offsetAt(range.getStartPosition());
		this.viewport.textModel.offsetAt(range.getEndPosition());
		this.selections.setSelections(SelectionSet.single(Selection.fromPositions(range.getStartPosition(), range.getEndPosition())));
		this.viewport.revealPosition(range.getStartPosition());
	}

	saveViewState(): CodeEditorViewState {
		return Object.freeze({
			selections: Object.freeze(this.selections.selections.selections.map(selection => Object.freeze({
				anchor: Object.freeze({ lineIndex: selection.selectionStartLineNumber - 1, columnIndex: selection.selectionStartColumn - 1 }),
				active: Object.freeze({ lineIndex: selection.positionLineNumber - 1, columnIndex: selection.positionColumn - 1 }),
			}))),
			primarySelectionIndex: this.selections.selections.primaryIndex,
			scrollPosition: Object.freeze({ ...this.viewport.currentLayout.scrollPosition }),
		});
	}

	restoreViewState(state: CodeEditorViewState): void {
		const selections = state.selections.map(selection => {
			const anchor = new Position((selection.anchor.lineIndex) + 1, (selection.anchor.columnIndex) + 1);
			const active = new Position((selection.active.lineIndex) + 1, (selection.active.columnIndex) + 1);
			this.viewport.textModel.offsetAt(anchor);
			this.viewport.textModel.offsetAt(active);
			return Selection.fromPositions(anchor, active);
		});
		this.selections.setSelections(SelectionSet.withPrimary(selections, state.primarySelectionIndex));
		this.viewport.scrollTo(state.scrollPosition);
	}

	getId(): string {
		return this.ownerId;
	}

	public getContribution<T extends CodeEditorContribution>(id: string): T | undefined {
		return this.contributions.get(id) as T | undefined;
	}
}

function validateOptions(options: CodeEditorWidgetOptions): void {
	if (!options || typeof options !== "object" || !isHTMLElement(options.container) || !options.model || !options.input || !options.languageId) {
		throw new TypeError("Code editor widget requires a container, input, language, and text model");
	}
	if (options.instantiationService !== undefined && typeof options.instantiationService.createInstance !== "function") {
		throw new TypeError("Code editor instantiation service must create instances");
	}
	if (options.onContributionError !== undefined && typeof options.onContributionError !== "function") {
		throw new TypeError("Code editor contribution error handler must be a function");
	}
}

export function isCodeEditorViewState(value: unknown): value is CodeEditorViewState {
	if (!value || typeof value !== 'object') return false;
	const state = value as Partial<CodeEditorViewState>;
	if (!isNonEmptyArray(state.selections)) return false;
	if (!isSafeInteger(state.primarySelectionIndex) || state.primarySelectionIndex! < 0 || state.primarySelectionIndex! >= state.selections.length) return false;
	if (!isViewScrollPosition(state.scrollPosition)) return false;
	return state.selections.every(selection => isViewPosition(selection?.anchor) && isViewPosition(selection?.active));
}

function executeEditorCommand<T>(emitter: Emitter<EditorCommandEvent>, commandId: string, operation: () => T): T {
	const result = operation();
	if (result && typeof (result as { readonly then?: unknown }).then === 'function') {
		return Promise.resolve(result).then(value => {
			emitter.fire(Object.freeze({ commandId }));
			return value;
		}) as T;
	}
	emitter.fire(Object.freeze({ commandId }));
	return result;
}

function editorLabel(input: EditorResourceInput): string {
	if (input.label?.trim()) return input.label;
	const path = decodeURIComponent(input.resource.path);
	return path.slice(path.lastIndexOf('/') + 1) || 'Text editor';
}

function resolveWrappingIndent(value: IEditorConstructionOptions['wrappingIndent']): WrappingIndent | undefined {
	switch (value) {
		case 'none': return WrappingIndent.None;
		case 'same': return WrappingIndent.Same;
		case 'indent': return WrappingIndent.Indent;
		case 'deepIndent': return WrappingIndent.DeepIndent;
		default: return undefined;
	}
}

function isViewPosition(value: unknown): value is CodeEditorViewPositionState {
	if (!value || typeof value !== 'object') return false;
	const position = value as Partial<CodeEditorViewPositionState>;
	return isSafeInteger(position.lineIndex) && position.lineIndex! >= 0 && isSafeInteger(position.columnIndex) && position.columnIndex! >= 0;
}

function isViewScrollPosition(value: unknown): value is CodeEditorViewState['scrollPosition'] {
	if (!value || typeof value !== 'object') return false;
	const position = value as Partial<CodeEditorViewState['scrollPosition']>;
	return isFiniteNumber(position.left) && position.left! >= 0 && isFiniteNumber(position.top) && position.top! >= 0;
}

function reportLanguageError(error: unknown): void {
	console.error('Editor language request failed', error);
}
