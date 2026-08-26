import { isHTMLElement } from "../../../../base/browser/dom.js";
import { getClientArea, type IDimension } from "../../../../base/browser/geometry.js";
import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import { type EditorSelectionController } from "../../../common/cursor/editorSelectionController.js";
import { type TextModel } from "../../../common/model/textModel.js";
import { EditorView, type EditorViewOptions, type EditorViewViewportOptions } from "../../view.js";
import { type EditorViewport } from "../../view/editorViewport.js";
import { KeyboardNavigationController, type KeyboardNavigationControllerOptions } from "../../controller/keyboardNavigationController.js";
import { MouseHandler, type MouseHandlerOptions } from "../../controller/mouseHandler.js";
import { InstantiationService, type IInstantiationService } from "../../../../platform/instantiation/common/instantiation.js";
import { CodeEditorContributions, type CodeEditorContributionDescription } from "./codeEditorContributions.js";

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

/**
 * Canonical browser editing surface for one Stanza text model and editor-local selection controller.
 *
 * Callers retain ownership of the model and selection controller. This component owns their DOM
 * projection plus native text input, keyboard navigation, and pointer selection. Optional
 * drop/paste behavior belongs to the host's contribution composition.
 */
export class CodeEditorWidget extends DisposableOwner {
	readonly ownerId: string;
	readonly view: EditorView;
	readonly viewport: EditorViewport;
	readonly contributions: CodeEditorContributions;

	constructor(options: CodeEditorWidgetOptions) {
		super();
		try {
			validateOptions(options);
			this.view = this.own(new EditorView({
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
			this.contributions = this.own(new CodeEditorContributions());
			this.contributions.initialize({
				model: options.model,
				selectionController: options.selectionController,
				viewport: this.viewport,
				view: this.view,
				placeholder: options.placeholder,
			}, options.instantiationService ?? new InstantiationService(), options.contributions, options.onContributionError);
			this.own(new KeyboardNavigationController(this.viewport, options.selectionController, options.keyboardNavigation));
			this.own(new MouseHandler(this.viewport, options.selectionController, options.mouseHandler));
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

	getId(): string {
		return this.ownerId;
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
