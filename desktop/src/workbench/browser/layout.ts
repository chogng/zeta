import { SplitView, type SplitViewOrientation } from "../../base/browser/ui/index.js";
import {
  ResettableDisposableGroup,
  DisposableOwner,
} from "../../base/common/lifecycle.js";
import { type WorkbenchPart } from "./part.js";

export const workbenchPartIds = ["titlebar", "statusbar", "sidebar", "session", "auxiliarybar", "editor"] as const;
export type WorkbenchPartId = typeof workbenchPartIds[number];

export interface SerializableGridPart {
  type: "part";
  partId: WorkbenchPartId;
  size?: string;
}

export interface SerializableGridSplit {
  type: "split";
  orientation: SplitViewOrientation;
  children: readonly SerializableGrid[];
  size?: string;
}

/** The serializable description of the workbench's nested horizontal and vertical regions. */
export type SerializableGrid = SerializableGridPart | SerializableGridSplit;

/** Builds and owns the runtime SplitView tree represented by a SerializableGrid. */
export class WorkbenchLayout extends DisposableOwner {
  #grid: SerializableGrid;
  readonly #layoutDisposables: ResettableDisposableGroup;

  constructor(
    private readonly container: Element,
    private readonly parts: ReadonlyMap<WorkbenchPartId, WorkbenchPart>,
    grid: SerializableGrid,
  ) {
    super();
    this.#grid = grid;
    this.#layoutDisposables = this.own(new ResettableDisposableGroup());
  }

  layout(): void {
    this.#layoutDisposables.clear();
    this.container.replaceChildren(this.createNode(this.#grid));
  }
  get serializableGrid(): SerializableGrid { return structuredClone(this.#grid); }

  restore(grid: SerializableGrid): void {
    this.#grid = grid;
    this.layout();
  }

  private createNode(node: SerializableGrid): HTMLElement {
    if (node.type === "part") return this.part(node.partId).element;
    const splitView = this.#layoutDisposables.add(
      new SplitView(node.orientation, this.container.ownerDocument),
    );
    splitView.element.classList.add("zeta-workbench-layout-split");
    for (const child of node.children) splitView.addPane(this.createNode(child), child.size ?? "1fr");
    return splitView.element;
  }

  private part(id: WorkbenchPartId): WorkbenchPart {
    const part = this.parts.get(id);
    if (!part) throw new Error(`Serializable grid references an unregistered workbench part: ${id}`);
    return part;
  }
}
