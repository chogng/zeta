import { addDisposableListener, stopEvent } from "../../../base/browser/dom.js";
import { DisposableOwner } from "../../../base/common/lifecycle.js";
import { resolveEditorIndentationOptions, type EditorIndentationOptions, type ResolvedEditorIndentationOptions } from "../common/editorIndentation.js";
import { type EditorSelectionController } from "../common/editorSelectionController.js";
import { createLineIndentCommand, EditorLineIndentDirection } from "../common/lineIndentCommands.js";
import { expandLineSelections } from "../common/lineSelection.js";
import { TextSelection, TextSelectionSet } from "../common/selection.js";
import { type AlphaEditorViewport } from "./alphaEditorViewport.js";

export interface AlphaEditingCommandControllerOptions {
  readonly indentation?: EditorIndentationOptions;
}

/** Routes synchronous document-wide and indentation shortcuts into Alpha common commands. */
export class AlphaEditingCommandController extends DisposableOwner {
  private readonly indentation: ResolvedEditorIndentationOptions;

  constructor(
    input: HTMLTextAreaElement,
    private readonly viewport: AlphaEditorViewport,
    private readonly selections: EditorSelectionController,
    options: AlphaEditingCommandControllerOptions = {},
  ) {
    super();
    if (viewport.textModel !== selections.textModel) {
      this.dispose();
      throw new TypeError("Alpha editing command dependencies must share one text model");
    }
    try {
      this.indentation = resolveEditorIndentationOptions(options.indentation);
    } catch (error) {
      this.dispose();
      throw error;
    }
    this.own(addDisposableListener(input, "keydown", event => this.handleKeydown(event)));
  }

  private handleKeydown(event: KeyboardEvent): void {
    if (event.defaultPrevented || event.isComposing || event.getModifierState("AltGraph")) return;
    if ((event.ctrlKey || event.metaKey) && !event.shiftKey && !event.altKey && event.key.toLowerCase() === "a") {
      stopEvent(event);
      const end = this.viewport.textModel.positionAt(this.viewport.textModel.createSnapshot().length);
      this.selections.setSelections(TextSelectionSet.single(TextSelection.from(this.viewport.textModel.positionAt(0), end)));
      this.viewport.revealPosition(end);
      return;
    }
    if ((event.ctrlKey || event.metaKey) && !event.shiftKey && !event.altKey && event.key.toLowerCase() === "l") {
      stopEvent(event);
      const next = expandLineSelections(this.viewport.textModel, this.selections.selections);
      this.selections.setSelections(next);
      this.viewport.revealPosition(next.primary.active);
      return;
    }
    if (event.key !== "Tab" || event.ctrlKey || event.altKey || event.metaKey) return;
    const hasRange = this.selections.selections.selections.some(selection => !selection.collapsed);
    if (!event.shiftKey && !hasRange) return;
    stopEvent(event);
    this.selections.execute(createLineIndentCommand(
      this.viewport.textModel,
      this.selections.selections,
      event.shiftKey ? EditorLineIndentDirection.Outdent : EditorLineIndentDirection.Indent,
      this.indentation,
    ));
    this.viewport.revealPosition(this.selections.selections.primary.active);
  }
}
