import { isHTMLElement } from "../../../../base/browser/dom.js";
import { getClientArea, type IDimension } from "../../../../base/browser/geometry.js";
import { Disposable } from "../../../../base/common/lifecycle.js";
import { type EditorSelectionController } from "../../../common/cursor/editorSelectionController.js";
import { TextSelection, TextSelectionSet } from "../../../common/core/selection.js";
import { TextPosition, type TextRange } from "../../../common/core/text.js";
import { type TextModel } from "../../../common/model/textModel.js";
import { type EditorScrollPosition } from "../../../common/viewModel.js";
import type { IContentWidget, IOverlayWidget, IViewZoneChangeAccessor } from '../../editorBrowser.js';
import { EditorView, type EditorViewOptions, type EditorViewViewportOptions } from "../../view.js";
import { type EditorViewport } from "../../view.js";
import { KeyboardNavigationController, type KeyboardNavigationControllerOptions } from "../../view/viewController.js";
import { MouseHandler, type MouseHandlerOptions } from "../../controller/mouseHandler.js";
import { ServiceContainer, type IInstantiationService } from "../../../../platform/instantiation/common/instantiation.js";
import { CodeEditorContributions, type CodeEditorContribution, type CodeEditorContributionDescription } from "./codeEditorContributions.js";
import { observableCodeEditor } from '../../observableCodeEditor.js';

export type CodeEditorWidgetViewportOptions = EditorViewViewportOptions;

export interface CodeEditorWidgetOptions extends Omit<EditorViewOptions, "container" | "model" | "selectionController" | "lineHeight"> {
	readonly container: HTMLElement;
	readonly model: TextModel;
	readonly selectionController: EditorSelectionController;
	readonly lineHeight: number;
	/** Optional placeholder text consumed by the registered placeholder contribution. */
	readonly placeholder?: string;
	/** Contributions to instantiate for this widget; defaults to the registered widget contributions. */
	readonly contributions?: readonly CodeEditorContributionDescription[];
	/** Optional service container used to construct contributions. */
	readonly instantiationService?: IInstantiationService;
	readonly onContributionError?: (error: unknown) => void;
	readonly keyboardNavigation?: KeyboardNavigationControllerOptions;
	readonly mouseHandler?: MouseHandlerOptions;
}

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
	private readonly selectionController: EditorSelectionController;
	readonly ownerId: string;
	readonly view: EditorView;
	readonly viewport: EditorViewport;
	readonly userInputEvents: EditorView['userInputEvents'];
	readonly contributions: CodeEditorContributions;

	constructor(options: CodeEditorWidgetOptions) {
		super();
		try {
			validateOptions(options);
			this.selectionController = options.selectionController;
			this.view = this._register(new EditorView({
				ownerId: options.ownerId,
				container: options.container,
				model: options.model,
				lineHeight: options.lineHeight,
				ariaLabel: options.ariaLabel,
				selectionController: options.selectionController,
				viewport: options.viewport,
				accessibilityService: options.accessibilityService,
				renderRichScreenReaderContent: options.renderRichScreenReaderContent,
				accessibilityPageSize: options.accessibilityPageSize,
				semanticTokenSource: options.semanticTokenSource,
				bracketColorizationSource: options.bracketColorizationSource,
				languageEditing: options.languageEditing,
				wordPattern: options.wordPattern,
			}));
			this.ownerId = this.view.ownerId;
			this.viewport = this.view.viewport;
			this.userInputEvents = this.view.userInputEvents;
			this._register(observableCodeEditor(this));
			this.contributions = this._register(new CodeEditorContributions());
			const instantiationService = this._register(options.instantiationService?.createChild() ?? new ServiceContainer());
			this.contributions.initialize({
				editor: this,
				model: options.model,
				selectionController: options.selectionController,
				viewport: this.viewport,
				view: this.view,
				placeholder: options.placeholder,
			}, instantiationService, options.contributions, options.onContributionError);
			this._register(new KeyboardNavigationController(this.viewport, options.selectionController, this.userInputEvents, options.keyboardNavigation));
			this._register(new MouseHandler(this.viewport, options.selectionController, options.mouseHandler));
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

	revealRange(range: TextRange): void {
		this.viewport.textModel.offsetAt(range.start);
		this.viewport.textModel.offsetAt(range.end);
		this.selectionController.setSelections(TextSelectionSet.single(TextSelection.from(range.start, range.end)));
		this.viewport.revealPosition(range.start);
	}

	saveViewState(): CodeEditorViewState {
		return Object.freeze({
			selections: Object.freeze(this.selectionController.selections.selections.map(selection => Object.freeze({
				anchor: Object.freeze({ lineIndex: selection.anchor.lineIndex, columnIndex: selection.anchor.columnIndex }),
				active: Object.freeze({ lineIndex: selection.active.lineIndex, columnIndex: selection.active.columnIndex }),
			}))),
			primarySelectionIndex: this.selectionController.selections.primaryIndex,
			scrollPosition: Object.freeze({ ...this.viewport.currentLayout.scrollPosition }),
		});
	}

	restoreViewState(state: CodeEditorViewState): void {
		const selections = state.selections.map(selection => {
			const anchor = TextPosition.at(selection.anchor.lineIndex, selection.anchor.columnIndex);
			const active = TextPosition.at(selection.active.lineIndex, selection.active.columnIndex);
			this.viewport.textModel.offsetAt(anchor);
			this.viewport.textModel.offsetAt(active);
			return TextSelection.from(anchor, active);
		});
		this.selectionController.setSelections(TextSelectionSet.withPrimary(selections, state.primarySelectionIndex));
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
	if (!options || typeof options !== "object" || !isHTMLElement(options.container) || !options.model || !options.selectionController) {
		throw new TypeError("Stanza code editor requires a container, text model, and selection controller");
	}
	if (options.selectionController.textModel !== options.model) {
		throw new TypeError("Stanza code editor model and selection controller must match");
	}
	if (options.instantiationService !== undefined && typeof options.instantiationService.createInstance !== "function") {
		throw new TypeError("Code editor instantiation service must create instances");
	}
	if (options.onContributionError !== undefined && typeof options.onContributionError !== "function") {
		throw new TypeError("Code editor contribution error handler must be a function");
	}
}
