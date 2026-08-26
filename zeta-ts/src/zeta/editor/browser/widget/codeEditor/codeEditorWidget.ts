import { isHTMLElement } from "../../../../base/browser/dom.js";
import { getClientArea, type IDimension } from "../../../../base/browser/geometry.js";
import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import { type EditorSelectionController } from "../../../common/cursor/editorSelectionController.js";
import { type TextModel } from "../../../common/model/textModel.js";
import { EditorViewport, type EditorViewportOptions } from "../../view/editorViewport.js";
import { KeyboardNavigationController, type KeyboardNavigationControllerOptions } from "../../controller/keyboardNavigationController.js";
import { MouseHandler, type MouseHandlerOptions } from "../../controller/mouseHandler.js";
import { EditorInputController, type InputControllerOptions } from "../../controller/inputController.js";
import { InstantiationService, type IInstantiationService } from "../../../../platform/instantiation/common/instantiation.js";
import { CodeEditorContributions, type CodeEditorContributionDescription } from "./codeEditorContributions.js";

export type CodeEditorWidgetViewportOptions = Omit<EditorViewportOptions, "container" | "model" | "lineHeight" | "ariaLabel" | "selectionController">;

export interface CodeEditorWidgetOptions {
	readonly container: HTMLElement;
	readonly model: TextModel;
	readonly selectionController: EditorSelectionController;
	readonly lineHeight: number;
	readonly ariaLabel?: string;
	/** Optional placeholder text consumed by the registered placeholder contribution. */
	readonly placeholder?: string;
	/** Contributions to instantiate for this widget; defaults to the registered widget contributions. */
	readonly contributions?: readonly CodeEditorContributionDescription[];
	/** Optional service container used to construct contributions. */
	readonly instantiationService?: IInstantiationService;
	readonly onContributionError?: (error: unknown) => void;
	readonly viewport?: CodeEditorWidgetViewportOptions;
	readonly input?: Omit<InputControllerOptions, "ariaLabel">;
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
	readonly viewport: EditorViewport;
	readonly input: EditorInputController;
	readonly contributions: CodeEditorContributions;

	constructor(options: CodeEditorWidgetOptions) {
		super();
		try {
			validateOptions(options);
			this.viewport = this.own(new EditorViewport({
				...options.viewport,
				container: options.container,
				model: options.model,
				lineHeight: options.lineHeight,
				ariaLabel: options.ariaLabel,
				selectionController: options.selectionController,
			}));
			this.input = this.own(new EditorInputController(this.viewport, options.selectionController, {
				...options.input,
				ariaLabel: options.ariaLabel,
			}));
			this.contributions = this.own(new CodeEditorContributions());
			this.contributions.initialize({
				model: options.model,
				selectionController: options.selectionController,
				viewport: this.viewport,
				input: this.input,
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
		this.input.focus();
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
