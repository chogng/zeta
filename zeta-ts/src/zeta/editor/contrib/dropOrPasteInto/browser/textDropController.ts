import { addDisposableListener, stopEvent } from "../../../../base/browser/dom.js";
import { registerEditorContribution } from "../../../browser/editorContribution.js";
import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import { createPasteTextCommand } from "../../../common/cursor/cursorTypeOperations.js";
import { type EditorSelectionController } from "../../../common/cursor/editorSelectionController.js";
import { TextSelection, TextSelectionSet } from "../../../common/core/selection.js";
import { type TextPosition } from "../../../common/core/text.js";
import { type EditorViewport } from "../../../browser/view/editorViewport.js";
import { readEditorHtmlText } from "../../clipboard/browser/clipboardController.js";
import { TEXT_FILE_TRANSFER_MAX_BYTES, selectTextFileTransfer } from "./textFileTransfer.js";

/** Routes external plain-text drops into one insertion at the viewport hit target. */
export class TextDropController extends DisposableOwner {
  private fileDropRequest = 0;
  private disposed = false;

  constructor(private readonly viewport: EditorViewport, private readonly selections: EditorSelectionController) {
    super();
    if (viewport.textModel !== selections.textModel) {
      this.dispose();
      throw new TypeError("Aster text drop dependencies must share one text model");
    }
    this.own(addDisposableListener<DragEvent>(viewport.element, "dragover", event => this.handleDragOver(event)));
    this.own(addDisposableListener<DragEvent>(viewport.element, "drop", event => this.handleDrop(event)));
    this.defer(() => {
      this.disposed = true;
      this.fileDropRequest += 1;
    });
  }

  private handleDragOver(event: DragEvent): void {
    if (this.selections.readOnly || event.defaultPrevented) return;
    if (!containsText(event.dataTransfer) && !selectTextFileTransfer(event.dataTransfer?.files ?? [])) return;
    event.preventDefault();
    if (event.dataTransfer) event.dataTransfer.dropEffect = "copy";
  }

  private handleDrop(event: DragEvent): void {
    if (this.selections.readOnly || event.defaultPrevented) return;
    const text = readDropText(event.dataTransfer, this.viewport.element.ownerDocument);
    const target = this.viewport.getNearestTargetAtClientPoint(event);
    if (!target) return;
    if (text.length === 0) {
      this.dropTextFile(event, target.position);
      return;
    }
    stopEvent(event);
    this.viewport.element.focus({ preventScroll: true });
    this.selections.execute(createPasteTextCommand(
      this.viewport.textModel,
      TextSelectionSet.single(TextSelection.collapsedAt(target.position)),
      text,
    ));
    this.viewport.revealPosition(this.selections.selections.primary.active);
  }

  private dropTextFile(event: DragEvent, position: TextPosition): void {
    const file = selectTextFileTransfer(event.dataTransfer?.files ?? []);
    if (!file) return;
    const model = this.viewport.textModel;
    const expectedVersion = model.version;
    const request = ++this.fileDropRequest;
    stopEvent(event);
    this.viewport.element.focus({ preventScroll: true });
    void file.text().then(text => {
      if (
        this.disposed ||
        request !== this.fileDropRequest ||
        text.length > TEXT_FILE_TRANSFER_MAX_BYTES ||
        model.version !== expectedVersion
      ) {
        return;
      }
      this.selections.execute(createPasteTextCommand(
        model,
        TextSelectionSet.single(TextSelection.collapsedAt(position)),
        text,
      ));
      this.viewport.revealPosition(this.selections.selections.primary.active);
    }).catch(() => {
      // The host supplied the file, but it could not be decoded as text.
    });
  }
}

registerEditorContribution({ id: "editor.contrib.dropOrPasteInto", install: context => {
  if (context.kind !== "text") return;
  context.own(new TextDropController(context.viewport, context.selections));
} });

function containsText(dataTransfer: DataTransfer | null): boolean {
  if (!dataTransfer) return false;
  const types = Array.from(dataTransfer.types);
  return types.includes("text/plain") || types.includes("text/html");
}

function readDropText(dataTransfer: DataTransfer | null, ownerDocument: Document): string {
  if (!dataTransfer) return "";
  try {
    const plainText = dataTransfer.getData("text/plain");
    if (plainText.length > 0) return plainText;
  } catch {
    // Rich text remains an inert text fallback when browsers omit plain text.
  }
  try {
    return readEditorHtmlText(dataTransfer.getData("text/html"), ownerDocument);
  } catch {
    return "";
  }
}
