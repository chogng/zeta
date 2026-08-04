import { addDisposableListener, stopEvent } from "../../../../../base/browser/dom.js";
import { DisposableOwner } from "../../../../../base/common/lifecycle.js";
import { type EditorSelectionController } from "../../../common/cursor/editorSelectionController.js";
import { createToggleLineCommentCommand } from "../common/lineCommentCommands.js";
import { type LanguageConfigurationSource } from "../../../common/languages/languageConfiguration.js";
import { type AlphaEditorViewport } from "../../../browser/view/editorViewport.js";

export interface AlphaLineCommentControllerOptions {
  readonly languageId: string;
  readonly configurations: LanguageConfigurationSource;
  readonly insertSpace?: boolean;
}

/** Routes the platform line-comment shortcut through Alpha's local command model. */
export class AlphaLineCommentController extends DisposableOwner {
  constructor(
    input: HTMLTextAreaElement,
    private readonly viewport: AlphaEditorViewport,
    private readonly selections: EditorSelectionController,
    private readonly options: AlphaLineCommentControllerOptions,
  ) {
    super();
    try {
      validateOptions(options);
      if (viewport.textModel !== selections.textModel) {
        throw new TypeError("Alpha line comment dependencies must share one text model");
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

function validateOptions(options: AlphaLineCommentControllerOptions): void {
  if (!options || typeof options !== "object" || typeof options.languageId !== "string" || options.languageId.length === 0) {
    throw new TypeError("Alpha line comment controller requires a language ID");
  }
  if (!options.configurations || typeof options.configurations.getLanguageConfiguration !== "function") {
    throw new TypeError("Alpha line comment controller requires language configurations");
  }
  if (options.insertSpace !== undefined && typeof options.insertSpace !== "boolean") {
    throw new TypeError("Alpha line comment insertSpace must be a boolean");
  }
}
