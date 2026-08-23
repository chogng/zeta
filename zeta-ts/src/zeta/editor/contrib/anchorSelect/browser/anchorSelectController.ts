import { addDisposableListener, stopEvent } from "../../../../base/browser/dom.js";
import { registerEditorContribution } from "../../../browser/editorContribution.js";
import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import { EditorCursorNavigationCommand, EditorCursorNavigationMode, navigateEditorCursors } from "../../../common/cursor/cursorNavigation.js";
import { type EditorSelectionController } from "../../../common/cursor/editorSelectionController.js";
import { TextSelection, TextSelectionSet } from "../../../common/core/selection.js";
import { type TextPosition } from "../../../common/core/text.js";
import { type EditorViewport } from "../../../browser/view/editorViewport.js";

/** Owns the editor-local anchor used by keyboard range expansion. */
export class AnchorSelectController extends DisposableOwner {
  private anchor: TextPosition | undefined;

  constructor(private readonly input: HTMLTextAreaElement, private readonly viewport: EditorViewport, private readonly selections: EditorSelectionController, private readonly wordPattern?: () => RegExp | undefined) {
    super();
    if (viewport.textModel !== selections.textModel) throw new TypeError("Aster anchor selection dependencies must share a text model");
    this.own(addDisposableListener(input, "keydown", event => this.handleKeydown(event), true));
    this.own(selections.onDidChange(() => {
      if (this.anchor && !this.selections.selections.primary.range.containsPosition(this.anchor)) this.anchor = undefined;
    }));
    this.defer(() => { this.anchor = undefined; });
  }

  get active(): boolean { return this.anchor !== undefined; }

  private handleKeydown(event: KeyboardEvent): void {
    if (event.defaultPrevented || event.isComposing || event.getModifierState("AltGraph")) return;
    const primaryModifier = event.ctrlKey || event.metaKey;
    if (primaryModifier && !event.altKey && !event.shiftKey && event.key.toLowerCase() === "k") {
      stopEvent(event, { immediate: true });
      this.anchor = this.selections.selections.primary.active;
      this.viewport.announceAccessibilityStatus("Anchor selection started");
      return;
    }
    if (!this.anchor) return;
    if (event.key === "Escape") {
      stopEvent(event, { immediate: true });
      this.anchor = undefined;
      this.viewport.announceAccessibilityStatus("Anchor selection cancelled");
      return;
    }
    if (event.key === "ArrowLeft" || event.key === "ArrowRight" || event.key === "ArrowUp" || event.key === "ArrowDown" || event.key === "Home" || event.key === "End") {
      stopEvent(event, { immediate: true });
      const command = anchorNavigationCommand(event);
      const result = navigateEditorCursors(this.viewport.textModel, this.selections.selections, { command, mode: EditorCursorNavigationMode.Move, ...(this.wordPattern ? { wordPattern: this.wordPattern() } : {}) });
      this.selections.setSelections(TextSelectionSet.single(TextSelection.from(this.anchor, result.selections.primary.active)));
      this.viewport.revealPosition(result.selections.primary.active);
    }
  }
}

registerEditorContribution({
  id: "editor.contrib.anchorSelect",
  install: context => {
    if (context.kind !== "text") return;
    context.own(new AnchorSelectController(context.textInput.element, context.viewport, context.selections, () => context.configurations.getLanguageConfiguration(context.languageId).wordPattern));
  },
});

function anchorNavigationCommand(event: KeyboardEvent): EditorCursorNavigationCommand {
  switch (event.key) {
    case "ArrowLeft": return event.ctrlKey || event.metaKey ? EditorCursorNavigationCommand.WordLeft : EditorCursorNavigationCommand.CharacterLeft;
    case "ArrowRight": return event.ctrlKey || event.metaKey ? EditorCursorNavigationCommand.WordRight : EditorCursorNavigationCommand.CharacterRight;
    case "ArrowUp": return EditorCursorNavigationCommand.LineUp;
    case "ArrowDown": return EditorCursorNavigationCommand.LineDown;
    case "Home": return EditorCursorNavigationCommand.LineStart;
    case "End": return EditorCursorNavigationCommand.LineEnd;
    default: throw new TypeError(`Unsupported anchor navigation key '${event.key}'`);
  }
}
