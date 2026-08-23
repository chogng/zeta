import { getClientArea, type IDimension } from "../../../../base/browser/geometry.js";
import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import { type EditorSelectionController } from "../../../common/cursor/editorSelectionController.js";
import { type TextModel } from "../../../common/model/textModel.js";
import { EditorViewport, type EditorViewportOptions } from "../../view/editorViewport.js";
import { KeyboardNavigationController, type KeyboardNavigationControllerOptions } from "../../input/keyboardNavigationController.js";
import { PointerSelectionController, type PointerSelectionControllerOptions } from "../../input/pointerSelectionController.js";
import { TextInputController, type TextInputControllerOptions } from "../../input/textInputController.js";
import { type IDisposable } from "../../../../base/common/lifecycle.js";

export type CodeEditorWidgetViewportOptions = Omit<EditorViewportOptions, "container" | "model" | "lineHeight" | "ariaLabel" | "selectionController">;

export interface CodeEditorWidgetOptions {
  readonly container: HTMLElement;
  readonly model: TextModel;
  readonly selectionController: EditorSelectionController;
  readonly lineHeight: number;
  readonly ariaLabel?: string;
  /** Compatibility seam for hosts that explicitly register placeholder presentation. */
  readonly placeholder?: string;
  readonly viewport?: CodeEditorWidgetViewportOptions;
  readonly textInput?: Omit<TextInputControllerOptions, "ariaLabel">;
  readonly keyboardNavigation?: KeyboardNavigationControllerOptions;
  readonly pointerSelection?: PointerSelectionControllerOptions;
}

/**
 * Canonical browser editing surface for one Aster text model and editor-local selection controller.
 *
 * Callers retain ownership of the model and selection controller. This component owns their DOM
 * projection plus native text input, keyboard navigation, and pointer selection. Optional
 * drop/paste behavior belongs to the host's contribution composition.
 */
export class CodeEditorWidget extends DisposableOwner {
  readonly viewport: EditorViewport;
  readonly textInput: TextInputController;

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
      this.textInput = this.own(new TextInputController(this.viewport, options.selectionController, {
        ...options.textInput,
        ariaLabel: options.ariaLabel,
      }));
      if (options.placeholder) {
        if (!placeholderFactory) throw new Error("Code editor placeholder requires the placeholder contribution");
        this.own(placeholderFactory(this.viewport, options.placeholder));
      }
      this.own(new KeyboardNavigationController(this.viewport, options.selectionController, options.keyboardNavigation));
      this.own(new PointerSelectionController(this.viewport, options.selectionController, options.pointerSelection));
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
    this.textInput.focus();
  }
}

export type CodeEditorPlaceholderFactory = (viewport: EditorViewport, placeholder: string) => IDisposable;

let placeholderFactory: CodeEditorPlaceholderFactory | undefined;

/** Registers optional placeholder presentation without a widget-to-contrib dependency. */
export function registerCodeEditorPlaceholderFactory(factory: CodeEditorPlaceholderFactory): void {
  if (typeof factory !== "function") throw new TypeError("Code editor placeholder factory must be a function");
  if (placeholderFactory && placeholderFactory !== factory) throw new Error("Code editor placeholder factory is already registered");
  placeholderFactory = factory;
}

function validateOptions(options: CodeEditorWidgetOptions): void {
  if (!options || typeof options !== "object" || !options.container || !options.model || !options.selectionController) {
    throw new TypeError("Aster code editor requires a container, text model, and selection controller");
  }
  if (options.selectionController.textModel !== options.model) {
    throw new TypeError("Aster code editor model and selection controller must match");
  }
}
