import {
  type IDimension,
  type IRectangle,
} from "../../geometry.js";
import { Emitter, type Event } from "../../../common/event.js";
import { DisposableOwner } from "../../../common/lifecycle.js";
import {
  type ISplitViewView,
  SplitView,
  type SplitViewLayoutPriority,
  type SplitViewOrientation,
} from "../splitview/splitview.js";

/** A two-dimensional leaf hosted by Grid. */
export interface IGridView {
  readonly element: HTMLElement;
  readonly minimumWidth: number;
  readonly maximumWidth: number;
  readonly minimumHeight: number;
  readonly maximumHeight: number;
  readonly onDidChange?: Event<void>;
  layout(bounds: IRectangle): void;
  setVisible?(visible: boolean): void;
}

export type GridDescriptor<TView extends IGridView> =
  | {
    readonly type: "leaf";
    readonly view: TView;
    readonly size: number;
    readonly visible?: boolean;
    readonly priority?: SplitViewLayoutPriority;
  }
  | {
    readonly type: "branch";
    readonly orientation: SplitViewOrientation;
    readonly size: number;
    readonly children: readonly GridDescriptor<TView>[];
    readonly priority?: SplitViewLayoutPriority;
  };

interface ParentLink {
  readonly branch: BranchNode;
  readonly index: number;
}

interface GridNodeHost {
  readonly ownerDocument: Document;
  ownSplitView(splitView: SplitView): SplitView;
  ownEvent(disposable: Disposable): void;
  createNode(descriptor: GridDescriptor<IGridView>): GridNode;
  handleSplitViewChange(): void;
}

abstract class GridNode {
  abstract readonly element: HTMLElement;
  parent: ParentLink | undefined;
  width = 0;
  height = 0;
  top = 0;
  left = 0;
  priority: SplitViewLayoutPriority = "normal";

  abstract get minimumWidth(): number;
  abstract get maximumWidth(): number;
  abstract get minimumHeight(): number;
  abstract get maximumHeight(): number;
  abstract isVisible(): boolean;
  abstract setDisplayed(visible: boolean): void;
  abstract layout(
    width: number,
    height: number,
    top: number,
    left: number,
  ): void;
}

class LeafNode<TView extends IGridView> extends GridNode {
  visible: boolean;

  constructor(readonly view: TView, visible: boolean) {
    super();
    this.visible = visible;
  }

  get element(): HTMLElement { return this.view.element; }
  get minimumWidth(): number { return this.view.minimumWidth; }
  get maximumWidth(): number { return this.view.maximumWidth; }
  get minimumHeight(): number { return this.view.minimumHeight; }
  get maximumHeight(): number { return this.view.maximumHeight; }
  isVisible(): boolean { return this.visible; }

  setDisplayed(visible: boolean): void {
    this.element.hidden = !visible;
    this.view.setVisible?.(visible);
  }

  layout(
    width: number,
    height: number,
    top: number,
    left: number,
  ): void {
    this.width = width;
    this.height = height;
    this.top = top;
    this.left = left;
    this.view.layout({ width, height, top, left });
  }
}

class BranchNode extends GridNode {
  readonly element: HTMLElement;
  readonly children: readonly GridNode[];
  readonly splitView: SplitView;

  constructor(
    readonly orientation: SplitViewOrientation,
    descriptors: readonly GridDescriptor<IGridView>[],
    priority: SplitViewLayoutPriority,
    host: GridNodeHost,
  ) {
    super();
    this.priority = priority;
    if (descriptors.length === 0) {
      throw new TypeError("Grid branches must contain at least one child");
    }
    this.splitView = host.ownSplitView(
      new SplitView(orientation, host.ownerDocument),
    );
    this.element = this.splitView.element;
    this.children = descriptors.map((descriptor) =>
      host.createNode(descriptor)
    );
    for (const [index, child] of this.children.entries()) {
      child.parent = { branch: this, index };
      this.splitView.addView(
        new AxisView(child, orientation),
        descriptorSizing(descriptors[index]!),
      );
    }
    host.ownEvent(this.splitView.onDidChangeViewSizes(() => {
      host.handleSplitViewChange();
    }));
  }

  get minimumWidth(): number {
    return this.#axisConstraint("minimumWidth", Math.max, 0);
  }

  get maximumWidth(): number {
    return this.#axisConstraint(
      "maximumWidth",
      Math.min,
      Number.POSITIVE_INFINITY,
    );
  }

  get minimumHeight(): number {
    return this.#axisConstraint("minimumHeight", Math.max, 0);
  }

  get maximumHeight(): number {
    return this.#axisConstraint(
      "maximumHeight",
      Math.min,
      Number.POSITIVE_INFINITY,
    );
  }

  isVisible(): boolean {
    return this.children.some((child) => child.isVisible());
  }

  setDisplayed(visible: boolean): void {
    this.element.hidden = !visible;
  }

  layout(
    width: number,
    height: number,
    top: number,
    left: number,
  ): void {
    this.width = width;
    this.height = height;
    this.top = top;
    this.left = left;
    if (this.orientation === "horizontal") {
      this.splitView.layout(width, height);
    } else {
      this.splitView.layout(height, width);
    }
  }

  #axisConstraint(
    property:
      | "minimumWidth"
      | "maximumWidth"
      | "minimumHeight"
      | "maximumHeight",
    orthogonalReducer: (left: number, right: number) => number,
    orthogonalInitial: number,
  ): number {
    const visible = this.children.filter((child) => child.isVisible());
    if (visible.length === 0) return 0;
    const isPrimary = this.orientation === "horizontal"
      ? property.endsWith("Width")
      : property.endsWith("Height");
    if (isPrimary) {
      return visible.reduce((total, child) => total + child[property], 0);
    }
    return visible.reduce(
      (result, child) => orthogonalReducer(result, child[property]),
      orthogonalInitial,
    );
  }
}

class AxisView implements ISplitViewView {
  constructor(
    readonly node: GridNode,
    readonly orientation: SplitViewOrientation,
  ) {}

  get element(): HTMLElement { return this.node.element; }
  get priority(): SplitViewLayoutPriority { return this.node.priority; }
  get minimumSize(): number {
    return this.orientation === "horizontal"
      ? this.node.minimumWidth
      : this.node.minimumHeight;
  }
  get maximumSize(): number {
    return this.orientation === "horizontal"
      ? this.node.maximumWidth
      : this.node.maximumHeight;
  }

  layout(size: number, offset: number, orthogonalSize: number): void {
    const parent = this.node.parent?.branch;
    const parentTop = parent?.top ?? 0;
    const parentLeft = parent?.left ?? 0;
    if (this.orientation === "horizontal") {
      this.node.layout(
        size,
        orthogonalSize,
        parentTop,
        parentLeft + offset,
      );
    } else {
      this.node.layout(
        orthogonalSize,
        size,
        parentTop + offset,
        parentLeft,
      );
    }
  }

  setVisible(visible: boolean): void {
    this.node.setDisplayed(visible);
  }
}

/**
 * A two-dimensional layout implemented as a nested tree of SplitViews.
 *
 * The descriptor is structural input only. Runtime sizes and visibility are
 * owned by the SplitViews so callers do not need to rebuild the tree.
 */
export class Grid<TView extends IGridView> extends DisposableOwner {
  readonly element: HTMLDivElement;
  readonly #root: GridNode;
  readonly #leaves = new Map<TView, LeafNode<TView>>();
  readonly #onDidChange = this.own(new Emitter<void>());
  #layoutWidth = 0;
  #layoutHeight = 0;
  #didLayout = false;
  #layingOut = false;

  readonly onDidChange: Event<void> = this.#onDidChange.event;

  constructor(
    descriptor: GridDescriptor<TView>,
    ownerDocument: Document = document,
  ) {
    super();
    validateDescriptor(descriptor, new Set());
    this.element = ownerDocument.createElement("div");
    this.element.className = "zeta-grid";
    this.defer(() => this.element.remove());
    const host: GridNodeHost = {
      ownerDocument,
      ownSplitView: (splitView) => this.own(splitView),
      ownEvent: (disposable) => {
        this.own(disposable);
      },
      createNode: (nodeDescriptor) =>
        this.#createNode(nodeDescriptor, host),
      handleSplitViewChange: () => {
        if (!this.#layingOut) this.#onDidChange.fire();
      },
    };
    this.#root = this.#createNode(
      descriptor as GridDescriptor<IGridView>,
      host,
    );
    this.element.append(this.#root.element);
    for (const [view, leaf] of this.#leaves) {
      leaf.setDisplayed(leaf.visible);
      if (view.onDidChange) {
        this.own(view.onDidChange(() => {
          if (this.#didLayout) {
            this.layout(this.#layoutWidth, this.#layoutHeight);
          }
        }));
      }
    }
  }

  layout(width: number, height: number): void {
    assertDimension(width, "width");
    assertDimension(height, "height");
    this.#layoutWidth = width;
    this.#layoutHeight = height;
    this.#didLayout = true;
    this.#layingOut = true;
    try {
      this.#root.layout(width, height, 0, 0);
    } finally {
      this.#layingOut = false;
    }
  }

  getViewSize(view: TView): IDimension {
    const leaf = this.#leaf(view);
    return { width: leaf.width, height: leaf.height };
  }

  resizeView(view: TView, dimension: IDimension): void {
    assertDimension(dimension.width, "view width");
    assertDimension(dimension.height, "view height");
    const leaf = this.#leaf(view);
    this.#resizeOnAxis(leaf, "horizontal", dimension.width);
    this.#resizeOnAxis(leaf, "vertical", dimension.height);
  }

  isViewVisible(view: TView): boolean {
    return this.#leaf(view).visible;
  }

  setViewVisible(view: TView, visible: boolean): void {
    const leaf = this.#leaf(view);
    if (leaf.visible === visible) return;
    leaf.visible = visible;
    leaf.setDisplayed(visible);
    let node: GridNode = leaf;
    while (node.parent) {
      const { branch, index } = node.parent;
      branch.splitView.setViewVisible(index, node.isVisible());
      node = branch;
    }
    if (this.#didLayout) {
      this.layout(this.#layoutWidth, this.#layoutHeight);
    }
    this.#onDidChange.fire();
  }

  #createNode(
    descriptor: GridDescriptor<IGridView>,
    host: GridNodeHost,
  ): GridNode {
    if (descriptor.type === "leaf") {
      const leaf = new LeafNode(descriptor.view, descriptor.visible !== false);
      leaf.priority = descriptor.priority ?? "normal";
      if (this.#leaves.has(descriptor.view as TView)) {
        throw new Error("Grid cannot contain the same view twice");
      }
      this.#leaves.set(descriptor.view as TView, leaf as LeafNode<TView>);
      return leaf;
    }
    return new BranchNode(
      descriptor.orientation,
      descriptor.children,
      descriptor.priority ?? "normal",
      host,
    );
  }

  #resizeOnAxis(
    leaf: LeafNode<TView>,
    orientation: SplitViewOrientation,
    size: number,
  ): void {
    let node: GridNode = leaf;
    while (node.parent) {
      const { branch, index } = node.parent;
      if (branch.orientation === orientation) {
        branch.splitView.resizeView(index, size);
        return;
      }
      node = branch;
    }
  }

  #leaf(view: TView): LeafNode<TView> {
    const leaf = this.#leaves.get(view);
    if (!leaf) throw new Error("Grid view is not registered");
    return leaf;
  }
}

function descriptorSizing(
  descriptor: GridDescriptor<IGridView>,
): number | { readonly type: "invisible"; readonly cachedVisibleSize: number } {
  return descriptor.type === "leaf" && descriptor.visible === false
    ? { type: "invisible", cachedVisibleSize: descriptor.size }
    : descriptor.size;
}

function validateDescriptor<TView extends IGridView>(
  descriptor: GridDescriptor<TView>,
  seenViews: Set<IGridView>,
): void {
  assertDimension(descriptor.size, "descriptor size");
  if (descriptor.type === "leaf") {
    if (seenViews.has(descriptor.view)) {
      throw new Error("Grid cannot contain the same view twice");
    }
    seenViews.add(descriptor.view);
    validateViewConstraints(descriptor.view);
    return;
  }
  if (
    descriptor.orientation !== "horizontal" &&
    descriptor.orientation !== "vertical"
  ) {
    throw new TypeError("Grid branch orientation is invalid");
  }
  if (descriptor.children.length === 0) {
    throw new TypeError("Grid branches must contain at least one child");
  }
  for (const child of descriptor.children) {
    validateDescriptor(child, seenViews);
  }
}

function assertDimension(value: number, name: string): void {
  if (!Number.isFinite(value) || value < 0) {
    throw new RangeError(`Grid ${name} must be a non-negative finite number`);
  }
}

function validateViewConstraints(view: IGridView): void {
  for (
    const [minimum, maximum, axis] of [
      [view.minimumWidth, view.maximumWidth, "width"],
      [view.minimumHeight, view.maximumHeight, "height"],
    ] as const
  ) {
    assertDimension(minimum, `view minimum ${axis}`);
    if (
      typeof maximum !== "number" ||
      Number.isNaN(maximum) ||
      maximum < minimum
    ) {
      throw new RangeError(
        `Grid view maximum ${axis} must be at least its minimum ${axis}`,
      );
    }
  }
}
