import "./media/diffEditorBreadcrumbs.css";
import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import { LineDiffKind, type LineDiffRow } from "../../../common/diff/lineDiff.js";
import { type DiffModel } from "../../../common/diff/diffModel.js";
import { type DiffEditorWidget } from "../../../browser/widget/diffEditor/diffEditorWidget.js";

/** Adds compact changed-hunk navigation to the Aster diff editor without touching diff computation. */
export class DiffEditorBreadcrumbsController extends DisposableOwner {
  private readonly element: HTMLElement;

  constructor(private readonly editor: DiffEditorWidget, private readonly model: DiffModel) {
    super();
    const document = editor.element.ownerDocument;
    this.element = document.createElement("nav");
    this.element.className = "aster-diff-editor-breadcrumbs";
    this.element.setAttribute("aria-label", "Diff changes");
    editor.element.append(this.element);
    this.defer(() => this.element.remove());
    this.own(model.onDidChange(() => this.render()));
    this.render();
  }

  private render(): void {
    const rows = this.model.diff?.rows ?? [];
    this.element.replaceChildren(...rows.flatMap((row, index) => row.kind === LineDiffKind.Unchanged ? [] : [this.createItem(row, index)]));
    this.element.hidden = this.element.childElementCount === 0;
  }

  private createItem(row: LineDiffRow, rowIndex: number): HTMLButtonElement {
    const button = this.element.ownerDocument.createElement("button");
    button.type = "button";
    button.className = "aster-diff-editor-breadcrumb";
    button.textContent = `${row.modifiedLineIndex === undefined ? "—" : row.modifiedLineIndex + 1}`;
    button.title = `Reveal change ${rowIndex + 1}`;
    button.addEventListener("click", () => {
      if (row.modifiedLineIndex !== undefined) this.editor.revealModifiedLine(row.modifiedLineIndex);
      else if (row.originalLineIndex !== undefined) this.editor.revealOriginalLine(row.originalLineIndex);
    });
    return button;
  }
}
