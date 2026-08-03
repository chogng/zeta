import { getClientArea, type IDimension } from "../../../../../base/browser/geometry.js";
import { DisposableOwner } from "../../../../../base/common/lifecycle.js";
import { type EditorSelectionController } from "../../../common/editorSelectionController.js";
import { type TextModel } from "../../../common/textModel.js";
import { AlphaEditorViewport, type AlphaEditorViewportOptions } from "../../alphaEditorViewport.js";
import { AlphaKeyboardNavigationController, type AlphaKeyboardNavigationControllerOptions } from "../../keyboardNavigationController.js";
import { AlphaPointerSelectionController, type AlphaPointerSelectionControllerOptions } from "../../pointerSelectionController.js";
import { AlphaTextInputController, type AlphaTextInputControllerOptions } from "../../textInputController.js";
import { AlphaTextDropController } from "../../textDropController.js";

export type CodeEditorWidgetViewportOptions = Omit<AlphaEditorViewportOptions, "container" | "model" | "lineHeight" | "ariaLabel" | "selectionController">;

export interface CodeEditorWidgetOptions {
  readonly container: HTMLElement;
  readonly model: TextModel;
  readonly selectionController: EditorSelectionController;
  readonly lineHeight: number;
  readonly ariaLabel?: string;
  readonly viewport?: CodeEditorWidgetViewportOptions;
  readonly textInput?: Omit<AlphaTextInputControllerOptions, "ariaLabel">;
  readonly keyboardNavigation?: AlphaKeyboardNavigationControllerOptions;
  readonly pointerSelection?: AlphaPointerSelectionControllerOptions;
}

/**
 * Canonical browser editing surface for one Alpha text model and editor-local selection controller.
 *
 * Callers retain ownership of the model and selection controller. This component owns their DOM
 * projection, native text input, keyboard and pointer navigation, and external text-drop adapter.
 */
export class CodeEditorWidget extends DisposableOwner {
  readonly viewport: AlphaEditorViewport;
  readonly textInput: AlphaTextInputController;

  constructor(options: CodeEditorWidgetOptions) {
    super();
    try {
      validateOptions(options);
      this.viewport = this.own(new AlphaEditorViewport({
        ...options.viewport,
        container: options.container,
        model: options.model,
        lineHeight: options.lineHeight,
        ariaLabel: options.ariaLabel,
        selectionController: options.selectionController,
      }));
      this.textInput = this.own(new AlphaTextInputController(this.viewport, options.selectionController, {
        ...options.textInput,
        ariaLabel: options.ariaLabel,
      }));
      this.own(new AlphaKeyboardNavigationController(this.viewport, options.selectionController, options.keyboardNavigation));
      this.own(new AlphaPointerSelectionController(this.viewport, options.selectionController, options.pointerSelection));
      this.own(new AlphaTextDropController(this.viewport, options.selectionController));
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

function validateOptions(options: CodeEditorWidgetOptions): void {
  if (!options || typeof options !== "object" || !options.container || !options.model || !options.selectionController) {
    throw new TypeError("Alpha code editor requires a container, text model, and selection controller");
  }
  if (options.selectionController.textModel !== options.model) {
    throw new TypeError("Alpha code editor model and selection controller must match");
  }
}
