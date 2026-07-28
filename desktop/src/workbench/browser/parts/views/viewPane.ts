import { DisposableOwner } from "../../../../base/common/lifecycle.js";

/** A titled, independently managed view hosted inside a workbench view container. */
export abstract class ViewPane extends DisposableOwner {
  readonly element: HTMLElement;
  readonly id: string;
  protected readonly titleElement: HTMLDivElement;
  protected readonly contentElement: HTMLDivElement;

  protected constructor(
    id: string,
    title: string,
    ownerDocument: Document = document,
  ) {
    super();
    const element = ownerDocument.createElement("section");
    this.element = element;
    this.defer(() => element.remove());
    element.className = "zeta-view-pane";
    element.dataset.viewId = id;
    this.id = id;
    this.titleElement = ownerDocument.createElement("div");
    this.titleElement.className = "zeta-view-pane-title";
    this.titleElement.textContent = title;
    this.contentElement = ownerDocument.createElement("div");
    this.contentElement.className = "zeta-view-pane-content";
    element.append(this.titleElement, this.contentElement);
  }

  setTitle(title: string): void { this.titleElement.textContent = title; }
}
