import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import { ViewPane } from "./viewPane.js";

/** A host that owns the ordered ViewPane instances displayed in one workbench region. */
export class ViewPaneContainer extends DisposableOwner {
  readonly element: HTMLElement;
  readonly id: string;
  #panes = new Map<string, ViewPane>();

  constructor(id: string, ownerDocument: Document = document) {
    super();
    const element = ownerDocument.createElement("div");
    this.element = element;
    this.defer(() => element.remove());
    element.className = "zeta-view-pane-container";
    element.dataset.viewContainerId = id;
    this.id = id;
    this.defer(() => {
      const panes = [...this.#panes.values()].reverse();
      this.#panes.clear();
      for (const pane of panes) pane.dispose();
    });
  }

  addPane(pane: ViewPane): void {
    if (this.#panes.has(pane.id)) throw new Error(`View pane is already registered: ${pane.id}`);
    this.#panes.set(pane.id, pane);
    this.element.append(pane.element);
  }

  removePane(id: string): ViewPane | undefined {
    const pane = this.#panes.get(id);
    if (!pane) return undefined;
    this.#panes.delete(id);
    pane.element.remove();
    return pane;
  }

  get panes(): readonly ViewPane[] { return [...this.#panes.values()]; }
}
