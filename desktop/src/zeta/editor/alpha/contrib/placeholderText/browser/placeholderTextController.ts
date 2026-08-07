import "./media/placeholderText.css";
import { DisposableOwner } from "../../../../../base/common/lifecycle.js";
import { type EditorViewport } from "../../../browser/view/editorViewport.js";

/** Presents a non-editable hint when the shared model is empty. */
export class PlaceholderTextController extends DisposableOwner {
  private readonly element: HTMLDivElement;

  constructor(private readonly viewport: EditorViewport, placeholder: string) {
    super();
    if (typeof placeholder !== "string" || placeholder.trim().length === 0) throw new TypeError("Alpha placeholder text must be non-empty");
    this.element = viewport.element.ownerDocument.createElement("div");
    this.element.className = "zeta-alpha-editor-placeholder-text";
    this.element.textContent = placeholder;
    this.element.setAttribute("aria-hidden", "true");
    viewport.element.append(this.element);
    this.defer(() => this.element.remove());
    this.own(viewport.textModel.onDidChange(() => this.update()));
    this.update();
  }

  private update(): void { this.element.hidden = this.viewport.textModel.length !== 0; }
}
