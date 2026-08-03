import { addDisposableListener, stopEvent } from "../../../base/browser/dom.js";
import { DisposableOwner } from "../../../base/common/lifecycle.js";
import { createToggleBlockCommentCommand } from "../common/blockCommentCommands.js";
import { type EditorSelectionController } from "../common/editorSelectionController.js";
import { type LanguageConfigurationSource } from "../language/common/languageConfiguration.js";
import { type AlphaEditorViewport } from "./alphaEditorViewport.js";

export interface AlphaBlockCommentControllerOptions {
  readonly languageId: string;
  readonly configurations: LanguageConfigurationSource;
}

/** Routes the platform block-comment shortcut through Alpha's local command model. */
export class AlphaBlockCommentController extends DisposableOwner {
  constructor(
    input: HTMLTextAreaElement,
    private readonly viewport: AlphaEditorViewport,
    private readonly selections: EditorSelectionController,
    private readonly options: AlphaBlockCommentControllerOptions,
  ) {
    super();
    try {
      validateOptions(options);
      if (viewport.textModel !== selections.textModel) {
        throw new TypeError("Alpha block comment dependencies must share one text model");
      }
      this.own(addDisposableListener(input, "keydown", event => this.handleKeydown(event)));
    } catch (error) {
      this.dispose();
      throw error;
    }
  }

  private handleKeydown(event: KeyboardEvent): void {
    if (event.defaultPrevented || event.isComposing || event.getModifierState("AltGraph")) return;
    if (event.ctrlKey || event.metaKey || !event.shiftKey || !event.altKey || event.key.toLowerCase() !== "a") return;
    const blockComment = this.options.configurations.getLanguageConfiguration(this.options.languageId).comments.blockComment;
    if (!blockComment) return;
    stopEvent(event);
    this.selections.execute(createToggleBlockCommentCommand(
      this.viewport.textModel,
      this.selections.selections,
      blockComment,
    ));
    this.viewport.revealPosition(this.selections.selections.primary.active);
  }
}

function validateOptions(options: AlphaBlockCommentControllerOptions): void {
  if (!options || typeof options !== "object" || typeof options.languageId !== "string" || options.languageId.length === 0) {
    throw new TypeError("Alpha block comment controller requires a language ID");
  }
  if (!options.configurations || typeof options.configurations.getLanguageConfiguration !== "function") {
    throw new TypeError("Alpha block comment controller requires language configurations");
  }
}
