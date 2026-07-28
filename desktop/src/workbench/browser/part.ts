import { DisposableOwner } from "../../base/common/lifecycle.js";

/** Base class for a persistent visual region in the browser workbench shell. */
export abstract class WorkbenchPart extends DisposableOwner {
  readonly element: HTMLElement;
  protected readonly titleElement: HTMLDivElement;
  protected readonly contentElement: HTMLDivElement;

  protected constructor(id: string, ownerDocument: Document) {
    super();
    const element = ownerDocument.createElement("section");
    this.element = element;
    this.defer(() => element.remove());
    element.className = `zeta-workbench-part zeta-workbench-${id}`;
    element.dataset.part = id;
    this.titleElement = ownerDocument.createElement("div");
    this.titleElement.className = "zeta-workbench-part-title";
    this.contentElement = ownerDocument.createElement("div");
    this.contentElement.className = "zeta-workbench-part-content";
    element.append(this.titleElement, this.contentElement);
  }
}
