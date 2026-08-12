import { addDisposableListener, stopEvent } from "../../../../base/browser/dom.js";
import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import { jumpToMatchingBrackets } from "../common/bracketNavigation.js";
import { type EditorSelectionController } from "../../../common/cursor/editorSelectionController.js";
import { type LanguageBracketMatcher } from "../common/bracketMatching.js";
import { type EditorViewport } from "../../../browser/view/editorViewport.js";

/** Routes the VS Code go-to-bracket shortcut through Alpha's lexical matcher. */
export class BracketNavigationController extends DisposableOwner {
  constructor(
    input: HTMLTextAreaElement,
    private readonly viewport: EditorViewport,
    private readonly selections: EditorSelectionController,
    private readonly matcher: LanguageBracketMatcher,
  ) {
    super();
    try {
      if (viewport.textModel !== selections.textModel || viewport.textModel !== matcher.textModel) {
        throw new TypeError("Alpha bracket navigation dependencies must share one text model");
      }
      this.own(addDisposableListener(input, "keydown", event => this.handleKeydown(event)));
    } catch (error) {
      this.dispose();
      throw error;
    }
  }

  private handleKeydown(event: KeyboardEvent): void {
    if (event.defaultPrevented || event.isComposing || event.getModifierState("AltGraph")) return;
    if ((!event.ctrlKey && !event.metaKey) || !event.shiftKey || event.altKey || event.key !== "\\") return;
    stopEvent(event);
    const next = jumpToMatchingBrackets(this.matcher, this.selections.selections);
    this.selections.setSelections(next);
    this.viewport.revealPosition(next.primary.active);
  }
}
