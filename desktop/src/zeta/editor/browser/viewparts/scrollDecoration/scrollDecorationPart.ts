import "./scrollDecoration.css";
import { h } from "../../../../base/browser/dom.js";
import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import { type EditorViewportLayout } from "../../../common/viewLayout/editorViewportModel.js";
import { type EditorViewPart } from "../viewPart.js";

/** Projects scroll shadows without owning the editor's scroll state. */
export class ScrollDecorationPart extends DisposableOwner implements EditorViewPart {
  private readonly topShadow: HTMLDivElement;
  private readonly bottomShadow: HTMLDivElement;
  private readonly element: HTMLDivElement;

  constructor(container: HTMLElement) {
    super();
    const ownerDocument = container.ownerDocument;
    this.element = h(ownerDocument, "div");
    this.topShadow = h(ownerDocument, "div");
    this.bottomShadow = h(ownerDocument, "div");
    this.element.className = "aster-editor-scroll-decoration";
    this.element.setAttribute("aria-hidden", "true");
    this.topShadow.className = "aster-editor-scroll-decoration-shadow top";
    this.bottomShadow.className = "aster-editor-scroll-decoration-shadow bottom";
    this.element.append(this.topShadow, this.bottomShadow);
    container.append(this.element);
  }

  render(layout: EditorViewportLayout): void {
    this.element.style.width = `${layout.viewportSize.width}px`;
    this.element.style.height = `${layout.viewportSize.height}px`;
    this.element.style.transform = `translate3d(${layout.scrollPosition.left}px, ${layout.scrollPosition.top}px, 0)`;
    this.topShadow.classList.toggle("visible", layout.scrollPosition.top > 0);
    this.bottomShadow.classList.toggle(
      "visible",
      layout.scrollPosition.top < layout.maximumScrollPosition.top,
    );
  }
}
