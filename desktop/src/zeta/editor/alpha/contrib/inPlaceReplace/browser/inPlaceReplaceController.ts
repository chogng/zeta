import { addDisposableListener, stopEvent } from "../../../../../base/browser/dom.js";
import { DisposableOwner } from "../../../../../base/common/lifecycle.js";
import { createEditorEditCommand } from "../../../common/commands/editorCommand.js";
import { type EditorSelectionController } from "../../../common/cursor/editorSelectionController.js";
import { TextRange } from "../../../common/core/text.js";
import { type AlphaEditorViewport } from "../../../browser/view/editorViewport.js";

/** Replaces the current selection with the next or previous matching occurrence. */
export class AlphaInPlaceReplaceController extends DisposableOwner {
  constructor(private readonly input: HTMLTextAreaElement, private readonly viewport: AlphaEditorViewport, private readonly selections: EditorSelectionController) {
    super();
    if (viewport.textModel !== selections.textModel) throw new TypeError("Alpha in-place replace dependencies must share a text model");
    this.own(addDisposableListener(input, "keydown", event => {
      if (event.defaultPrevented || event.isComposing || !event.shiftKey || event.key !== "Enter" || (!event.ctrlKey && !event.metaKey)) return;
      stopEvent(event);
      this.replace(event.altKey ? -1 : 1);
    }, true));
  }

  replace(direction: 1 | -1): boolean {
    const model = this.viewport.textModel;
    const selection = this.selections.selections.primary;
    if (selection.range.empty) return false;
    const value = model.getTextInRange(selection.range);
    if (value.length === 0) return false;
    const source = model.getText();
    const start = model.offsetAt(selection.range.start);
    const end = model.offsetAt(selection.range.end);
    const occurrence = findOccurrence(source, value, direction, direction > 0 ? end : start);
    if (!occurrence) return false;
    const range = TextRange.from(model.positionAt(occurrence.start), model.positionAt(occurrence.end));
    const command = createEditorEditCommand(model, this.selections.selections, [{ range, text: value }]);
    if (!command) return false;
    this.selections.execute(command);
    this.viewport.revealPosition(range.start);
    return true;
  }
}

function findOccurrence(source: string, value: string, direction: 1 | -1, anchor: number): { readonly start: number; readonly end: number } | undefined {
  if (direction > 0) {
    const next = source.indexOf(value, anchor);
    const wrapped = next >= 0 ? next : source.indexOf(value, 0);
    return wrapped >= 0 ? { start: wrapped, end: wrapped + value.length } : undefined;
  }
  const before = source.slice(0, Math.max(0, anchor - value.length + 1));
  const previous = before.lastIndexOf(value);
  const wrapped = previous >= 0 ? previous : source.lastIndexOf(value);
  return wrapped >= 0 ? { start: wrapped, end: wrapped + value.length } : undefined;
}
