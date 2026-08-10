import "./media/placeholderText.css";
import { DisposableOwner } from "../../../../../base/common/lifecycle.js";
import { type EditorViewport } from "../../../browser/view/editorViewport.js";
import { TextPosition } from "../../../common/core/text.js";

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
    this.own(viewport.onDidChangeLayout(() => this.updateLayout()));
    this.update();
    this.updateLayout();
  }

  private update(): void { this.element.hidden = this.viewport.textModel.length !== 0; }

  private updateLayout(): void {
    const position = this.viewport.getPositionContentCoordinates(TextPosition.at(0, 0));
    this.element.style.left = `${position.left}px`;
    this.element.style.top = `${position.top}px`;
    this.element.style.lineHeight = `${position.height}px`;
  }
}
