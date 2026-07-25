import { Component } from "../../../../base/browser/ui/index.js";
import { ViewPane } from "./viewPane.js";

/** A host that owns the ordered ViewPane instances displayed in one workbench region. */
export class ViewPaneContainer extends Component<HTMLElement> {
  readonly id: string;
  #panes = new Map<string, ViewPane>();

  constructor(id: string) {
    const element = document.createElement("div");
    element.className = "zeta-view-pane-container";
    element.dataset.viewContainerId = id;
    super(element);
    this.id = id;
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
