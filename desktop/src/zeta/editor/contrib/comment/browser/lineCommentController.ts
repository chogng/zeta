import { addDisposableListener, stopEvent } from "../../../../base/browser/dom.js";
import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import { type EditorSelectionController } from "../../../common/cursor/editorSelectionController.js";
import { createToggleLineCommentCommand } from "../common/lineCommentCommands.js";
import { type LanguageConfigurationSource } from "../../../common/languages/languageConfiguration.js";
import { type EditorViewport } from "../../../browser/view/editorViewport.js";

export interface LineCommentControllerOptions {
  readonly languageId: string;
  readonly configurations: LanguageConfigurationSource;
  readonly insertSpace?: boolean;
}

/** Routes the platform line-comment shortcut through Aster's local command model. */
export class LineCommentController extends DisposableOwner {
  constructor(
    input: HTMLTextAreaElement,
    private readonly viewport: EditorViewport,
    private readonly selections: EditorSelectionController,
    private readonly options: LineCommentControllerOptions,
  ) {
    super();
    try {
      validateOptions(options);
      if (viewport.textModel !== selections.textModel) {
        throw new TypeError("Aster line comment dependencies must share one text model");
      }
      this.own(addDisposableListener(input, "keydown", event => this.handleKeydown(event)));
    } catch (error) {
      this.dispose();
      throw error;
    }
  }

  private handleKeydown(event: KeyboardEvent): void {
    if (event.defaultPrevented || event.isComposing || event.getModifierState("AltGraph")) return;
    if ((!event.ctrlKey && !event.metaKey) || event.shiftKey || event.altKey || event.key !== "/") return;
    const lineComment = this.options.configurations.getLanguageConfiguration(this.options.languageId).comments.lineComment;
    if (!lineComment) return;
    stopEvent(event);
    this.selections.execute(createToggleLineCommentCommand(
      this.viewport.textModel,
      this.selections.selections,
      {
        lineComment,
        insertSpace: this.options.insertSpace,
      },
    ));
    this.viewport.revealPosition(this.selections.selections.primary.active);
  }
}

function validateOptions(options: LineCommentControllerOptions): void {
  if (!options || typeof options !== "object" || typeof options.languageId !== "string" || options.languageId.length === 0) {
    throw new TypeError("Aster line comment controller requires a language ID");
  }
  if (!options.configurations || typeof options.configurations.getLanguageConfiguration !== "function") {
    throw new TypeError("Aster line comment controller requires language configurations");
  }
  if (options.insertSpace !== undefined && typeof options.insertSpace !== "boolean") {
    throw new TypeError("Aster line comment insertSpace must be a boolean");
  }
}
