import { addDisposableListener, stopEvent } from "../../../../base/browser/dom.js";
import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import { createRemoveMatchingBracketsCommand } from "../common/bracketEditing.js";
import { type EditorSelectionController } from "../../common/editorSelectionController.js";
import { type LanguageBracketMatcher } from "../common/languageBracketMatcher.js";
import { type AlphaEditorViewport } from "../../browser/alphaEditorViewport.js";

/** Routes the VS Code remove-brackets chord through Alpha's lexical bracket matcher. */
export class AlphaBracketEditingController extends DisposableOwner {
  constructor(
    input: HTMLTextAreaElement,
    private readonly viewport: AlphaEditorViewport,
    private readonly selections: EditorSelectionController,
    private readonly matcher: LanguageBracketMatcher,
  ) {
    super();
    try {
      if (viewport.textModel !== selections.textModel || viewport.textModel !== matcher.textModel) {
        throw new TypeError("Alpha bracket editing dependencies must share one text model");
      }
      this.own(addDisposableListener(input, "keydown", event => this.handleKeydown(event)));
    } catch (error) {
      this.dispose();
      throw error;
    }
  }

  private handleKeydown(event: KeyboardEvent): void {
    if (event.defaultPrevented || event.isComposing || event.getModifierState("AltGraph")) return;
    if ((!event.ctrlKey && !event.metaKey) || !event.altKey || event.shiftKey || event.key !== "Backspace") return;
    const command = createRemoveMatchingBracketsCommand(this.matcher, this.selections.selections);
    if (!command) return;
    stopEvent(event);
    this.selections.execute(command);
    this.viewport.revealPosition(this.selections.selections.primary.active);
  }
}
