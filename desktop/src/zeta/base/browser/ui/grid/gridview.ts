import { type IDimension } from "../../geometry.js";
import { Emitter, type Event } from "../../../common/event.js";
import { DisposableOwner, ResettableDisposableGroup } from "../../../common/lifecycle.js";
import { type ISplitViewView, SplitView, type SplitViewLayoutPriority, type SplitViewOrientation } from "../splitview/splitview.js";
import { assertChildIndex, assertDimension, assertInsertionIndex, descriptorNode, descriptorSizing, deserializeGridViewDescriptor, isSerializableView, normalizeDescriptor, normalizeRootDescriptor, orthogonal, replaceDescriptorNode, splitLocation, type GridLocation, type GridViewDescriptor, type GridViewSizing, type ISerializableView, type IView, type IViewDeserializer, type SerializedGridViewDescriptor, validateDescriptor, validateSerializedGridViewDescriptor, validateViewConstraints } from "./gridviewDescriptor.js";

export { type GridLocation, type GridViewDescriptor, type GridViewSizing, type ISerializableView, type IView, type IViewDeserializer, type SerializedGridViewDescriptor } from "./gridviewDescriptor.js";

interface ParentLink {
  readonly branch: BranchNode;
  readonly index: number;
}

interface GridNodeHost {
  readonly ownerDocument: Document;
  ownSplitView(splitView: SplitView): SplitView;
  ownEvent(disposable: Disposable): void;
  createNode(descriptor: GridViewDescriptor<IView>): GridNode;
  handleSplitViewChange(): void;
}

abstract class GridNode {
  constructor(readonly initialSize: number) {}

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
  abstract layout(width: number, height: number, top: number, left: number): void;
}

class LeafNode extends GridNode {
  constructor(
    readonly view: IView,
    public visible: boolean,
    initialSize: number,
  ) {
    super(initialSize);
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

  layout(width: number, height: number, top: number, left: number): void {
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
    descriptors: readonly GridViewDescriptor<IView>[],
    priority: SplitViewLayoutPriority,
    initialSize: number,
    host: GridNodeHost,
  ) {
    super(initialSize);
    this.priority = priority;
    this.splitView = host.ownSplitView(new SplitView(orientation, host.ownerDocument));
    this.element = this.splitView.element;
    this.children = descriptors.map((descriptor) => host.createNode(descriptor));
    for (const [index, child] of this.children.entries()) {
      child.parent = { branch: this, index };
      this.splitView.addView(new AxisView(child, orientation), descriptorSizing(descriptors[index]!));
    }
    host.ownEvent(this.splitView.onDidChangeViewSizes(() => host.handleSplitViewChange()));
  }

  get minimumWidth(): number {
    return this.axisConstraint("minimumWidth", Math.max, 0);
  }

  get maximumWidth(): number {
    return this.axisConstraint("maximumWidth", Math.min, Number.POSITIVE_INFINITY);
  }

  get minimumHeight(): number {
    return this.axisConstraint("minimumHeight", Math.max, 0);
  }

  get maximumHeight(): number {
    return this.axisConstraint("maximumHeight", Math.min, Number.POSITIVE_INFINITY);
  }

  isVisible(): boolean {
    return this.children.some((child) => child.isVisible());
  }

  setDisplayed(visible: boolean): void {
    this.element.hidden = !visible;
  }

  layout(width: number, height: number, top: number, left: number): void {
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

  private axisConstraint(
    property: "minimumWidth" | "maximumWidth" | "minimumHeight" | "maximumHeight",
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
      this.node.layout(size, orthogonalSize, parentTop, parentLeft + offset);
    } else {
      this.node.layout(orthogonalSize, size, parentTop + offset, parentLeft);
    }
  }

  setVisible(visible: boolean): void {
    this.node.setDisplayed(visible);
  }
}

/**
 * Index-addressed two-dimensional layout engine.
 *
 * GridView owns the nested SplitView tree. Callers that identify leaves by
 * object identity should use Grid instead.
 */
export class GridView extends DisposableOwner {
  readonly element: HTMLDivElement;
  private readonly treeResources = this.own(new ResettableDisposableGroup());
  private readonly leaves = new Map<IView, LeafNode>();
  private readonly _onDidChange = this.own(new Emitter<void>());
  private root!: BranchNode;
  private layoutWidth = 0;
  private layoutHeight = 0;
  private didLayout = false;
  private layingOut = false;

  readonly onDidChange: Event<void> = this._onDidChange.event;

  static deserialize<TView extends ISerializableView>(
    descriptor: SerializedGridViewDescriptor,
    deserializer: IViewDeserializer<TView>,
    ownerDocument: Document = document,
  ): GridView {
    validateSerializedGridViewDescriptor(descriptor);
    return new GridView(
      deserializeGridViewDescriptor(descriptor, deserializer),
      ownerDocument,
    );
  }

  constructor(
    descriptor: GridViewDescriptor<IView>,
    ownerDocument: Document = document,
  ) {
    super();
    this.element = ownerDocument.createElement("div");
    this.element.className = "zeta-grid zeta-grid-view";
    this.defer(() => this.element.remove());
    this.rebuild(normalizeRootDescriptor(descriptor));
  }

  get orientation(): SplitViewOrientation {
    return this.root.orientation;
  }

  get width(): number { return this.root.width; }
  get height(): number { return this.root.height; }
  get minimumWidth(): number { return this.root.minimumWidth; }
  get maximumWidth(): number { return this.root.maximumWidth; }
  get minimumHeight(): number { return this.root.minimumHeight; }
  get maximumHeight(): number { return this.root.maximumHeight; }

  layout(width: number, height: number): void {
    assertDimension(width, "width");
    assertDimension(height, "height");
    this.layoutWidth = width;
    this.layoutHeight = height;
    this.didLayout = true;
    this.layingOut = true;
    try {
      this.root.layout(width, height, 0, 0);
    } finally {
      this.layingOut = false;
    }
  }

  addView(view: IView, sizing: GridViewSizing, location: GridLocation): void {
    if (this.leaves.has(view)) {
      throw new Error("GridView cannot contain the same view twice");
    }
    validateViewConstraints(view);
    const [parentLocation, index] = splitLocation(location);
    const runtimeParent = this.node(parentLocation);
    let descriptor = this.snapshot();
    const parent = descriptorNode(descriptor, parentLocation);
    const leaf = createAddedLeaf(view, sizing, runtimeParent);
    if (parent.type === "branch") {
      assertInsertionIndex(index, parent.children.length);
      const children = [...parent.children];
      if (typeof sizing !== "number" && sizing.type === "split") {
        const target = children[sizing.index];
        if (!target) throw new RangeError("GridView split target is out of range");
        const splitSize = target.size / 2;
        children[sizing.index] = { ...target, size: splitSize };
        leaf.size = splitSize;
      }
      children.splice(index, 0, leaf);
      descriptor = replaceDescriptorNode(
        descriptor,
        parentLocation,
        { ...parent, children },
      );
    } else {
      if (index !== 0 && index !== 1) {
        throw new RangeError("GridView nested insertion index must be 0 or 1");
      }
      const orientation = orthogonal(runtimeParent.parent?.branch.orientation ?? this.root.orientation);
      const existingSize = orientation === "horizontal"
        ? runtimeParent.width
        : runtimeParent.height;
      const childSize = existingSize > 0 ? existingSize : parent.size;
      const existing = { ...parent, size: childSize };
      if (typeof sizing !== "number" && sizing.type === "split") {
        existing.size = childSize / 2;
        leaf.size = childSize / 2;
      }
      const children = index === 0 ? [leaf, existing] : [existing, leaf];
      descriptor = replaceDescriptorNode(descriptor, parentLocation, {
        type: "branch",
        orientation,
        size: parent.size,
        children,
        priority: parent.priority,
      });
    }
    this.rebuild(descriptor);
    this._onDidChange.fire();
  }

  removeView(location: GridLocation): IView {
    const leaf = this.leafAt(location);
    if (this.leaves.size === 1) {
      throw new Error("GridView cannot remove its last view");
    }
    let descriptor = this.snapshot();
    const [parentLocation, index] = splitLocation(location);
    const parent = descriptorNode(descriptor, parentLocation);
    if (parent.type !== "branch" || parent.children[index]?.type !== "leaf") {
      throw new Error("GridView location does not identify a leaf");
    }
    const children = parent.children.filter((_, childIndex) => childIndex !== index);
    descriptor = replaceDescriptorNode(
      descriptor,
      parentLocation,
      { ...parent, children },
    );
    this.rebuild(normalizeDescriptor(descriptor, true));
    leaf.view.element.remove();
    this._onDidChange.fire();
    return leaf.view;
  }

  moveView(parentLocation: GridLocation, from: number, to: number): void {
    let descriptor = this.snapshot();
    const parent = descriptorNode(descriptor, parentLocation);
    if (parent.type !== "branch") {
      throw new Error("GridView parent location does not identify a branch");
    }
    assertChildIndex(from, parent.children.length);
    assertChildIndex(to, parent.children.length);
    if (from === to) return;
    const children = [...parent.children];
    const [moved] = children.splice(from, 1);
    children.splice(to, 0, moved!);
    descriptor = replaceDescriptorNode(
      descriptor,
      parentLocation,
      { ...parent, children },
    );
    this.rebuild(descriptor);
    this._onDidChange.fire();
  }

  getViewLocation(view: IView): GridLocation {
    const leaf = this.leaf(view);
    const location: number[] = [];
    let node: GridNode = leaf;
    while (node.parent) {
      location.unshift(node.parent.index);
      node = node.parent.branch;
    }
    return location;
  }

  getViews(): readonly IView[] {
    return [...this.leaves.keys()];
  }

  getViewSize(location?: GridLocation): IDimension {
    const node = location ? this.node(location) : this.root;
    if (node instanceof LeafNode && !node.visible && node.parent) {
      const { branch, index } = node.parent;
      const primarySize =
        branch.splitView.getViewCachedVisibleSize(index) ??
        branch.splitView.getViewSize(index);
      return branch.orientation === "horizontal"
        ? { width: primarySize, height: branch.height }
        : { width: branch.width, height: primarySize };
    }
    return { width: node.width, height: node.height };
  }

  getViewCachedVisibleSize(location: GridLocation): number | undefined {
    const leaf = this.leafAt(location);
    if (!leaf.parent) return undefined;
    return leaf.parent.branch.splitView.getViewCachedVisibleSize(leaf.parent.index);
  }

  resizeView(location: GridLocation, dimension: Partial<IDimension>): void {
    const leaf = this.leafAt(location);
    if (dimension.width !== undefined) {
      assertDimension(dimension.width, "view width");
      this.resizeOnAxis(leaf, "horizontal", dimension.width);
    }
    if (dimension.height !== undefined) {
      assertDimension(dimension.height, "view height");
      this.resizeOnAxis(leaf, "vertical", dimension.height);
    }
  }

  isViewVisible(location: GridLocation): boolean {
    return this.leafAt(location).visible;
  }

  setViewVisible(location: GridLocation, visible: boolean): void {
    const leaf = this.leafAt(location);
    if (leaf.visible === visible) return;
    leaf.visible = visible;
    let node: GridNode = leaf;
    while (node.parent) {
      const { branch, index } = node.parent;
      branch.splitView.setViewVisible(index, node.isVisible());
      node = branch;
    }
    if (this.didLayout) this.layout(this.layoutWidth, this.layoutHeight);
    this._onDidChange.fire();
  }

  serialize(): SerializedGridViewDescriptor {
    return serializeGridNode(this.root, this.didLayout);
  }

  private rebuild(descriptor: GridViewDescriptor<IView>): void {
    validateDescriptor(descriptor, new Set(), undefined);
    this.treeResources.clear();
    this.leaves.clear();
    const host: GridNodeHost = {
      ownerDocument: this.element.ownerDocument,
      ownSplitView: (splitView) => this.treeResources.add(splitView),
      ownEvent: (disposable) => {
        this.treeResources.add(disposable);
      },
      createNode: (nodeDescriptor) => this.createNode(nodeDescriptor, host),
      handleSplitViewChange: () => {
        if (!this.layingOut) this._onDidChange.fire();
      },
    };
    const root = this.createNode(descriptor, host);
    if (!(root instanceof BranchNode)) {
      throw new TypeError("GridView root must be a branch");
    }
    this.root = root;
    this.element.append(root.element);
    for (const [view, leaf] of this.leaves) {
      leaf.setDisplayed(leaf.visible);
      if (view.onDidChange) {
        this.treeResources.add(view.onDidChange(() => {
          if (this.didLayout) this.layout(this.layoutWidth, this.layoutHeight);
        }));
      }
    }
    if (this.didLayout) this.layout(this.layoutWidth, this.layoutHeight);
  }

  private createNode(
    descriptor: GridViewDescriptor<IView>,
    host: GridNodeHost,
  ): GridNode {
    if (descriptor.type === "leaf") {
      const leaf = new LeafNode(descriptor.view, descriptor.visible !== false, descriptor.size);
      leaf.priority = descriptor.priority ?? "normal";
      this.leaves.set(descriptor.view, leaf);
      return leaf;
    }
    return new BranchNode(
      descriptor.orientation,
      descriptor.children,
      descriptor.priority ?? "normal",
      descriptor.size,
      host,
    );
  }

  private resizeOnAxis(
    leaf: LeafNode,
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

  private node(location: GridLocation): GridNode {
    let node: GridNode = this.root;
    for (const index of location) {
      if (!(node instanceof BranchNode)) {
        throw new Error("GridView location traverses through a leaf");
      }
      assertChildIndex(index, node.children.length);
      node = node.children[index]!;
    }
    return node;
  }

  private leafAt(location: GridLocation): LeafNode {
    const node = this.node(location);
    if (!(node instanceof LeafNode)) {
      throw new Error("GridView location does not identify a leaf");
    }
    return node;
  }

  private leaf(view: IView): LeafNode {
    const leaf = this.leaves.get(view);
    if (!leaf) throw new Error("Grid view is not registered");
    return leaf;
  }

  private snapshot(): GridViewDescriptor<IView> {
    return snapshotGridNode(this.root, this.didLayout);
  }
}

function snapshotGridNode(node: GridNode, didLayout: boolean): GridViewDescriptor<IView> {
  const size = gridNodeSize(node, didLayout);
  if (node instanceof LeafNode) {
    return {
      type: "leaf",
      view: node.view,
      size,
      visible: node.visible,
      priority: node.priority,
    };
  }
  if (!(node instanceof BranchNode)) {
    throw new TypeError("GridView contains an unsupported node");
  }
  return {
    type: "branch",
    orientation: node.orientation,
    size,
    children: node.children.map((child) => snapshotGridNode(child, didLayout)),
    priority: node.priority,
  };
}

function serializeGridNode(node: GridNode, didLayout: boolean): SerializedGridViewDescriptor {
  const size = gridNodeSize(node, didLayout);
  if (node instanceof LeafNode) {
    if (!isSerializableView(node.view)) {
      throw new TypeError("SerializableGrid contains a non-serializable view");
    }
    return {
      type: "leaf",
      data: node.view.toJSON(),
      size,
      visible: node.visible,
      priority: node.priority,
    };
  }
  if (!(node instanceof BranchNode)) {
    throw new TypeError("SerializableGrid contains an unsupported node");
  }
  return {
    type: "branch",
    orientation: node.orientation,
    size,
    children: node.children.map((child) => serializeGridNode(child, didLayout)),
    priority: node.priority,
  };
}

function gridNodeSize(node: GridNode, didLayout: boolean): number {
  if (node.parent) {
    const { branch, index } = node.parent;
    return branch.splitView.getViewCachedVisibleSize(index) ?? branch.splitView.getViewSize(index);
  }
  if (!didLayout) return node.initialSize;
  return node instanceof BranchNode && node.orientation === "vertical" ? node.height : node.width;
}

function createAddedLeaf(
  view: IView,
  sizing: GridViewSizing,
  parent: GridNode,
): {
  type: "leaf";
  view: IView;
  size: number;
  visible: boolean;
  priority: SplitViewLayoutPriority;
} {
  if (typeof sizing === "number") {
    assertDimension(sizing, "added view size");
    return { type: "leaf", view, size: sizing, visible: true, priority: "normal" };
  }
  if (sizing.type === "invisible") {
    assertDimension(sizing.cachedVisibleSize, "added view cached visible size");
    return {
      type: "leaf",
      view,
      size: sizing.cachedVisibleSize,
      visible: false,
      priority: "normal",
    };
  }
  const primarySize = parent instanceof BranchNode
    ? (parent.orientation === "horizontal" ? parent.width : parent.height)
    : Math.max(parent.width, parent.height);
  const divisor = parent instanceof BranchNode ? parent.children.length + 1 : 2;
  return {
    type: "leaf",
    view,
    size: primarySize > 0 ? primarySize / divisor : parent.initialSize / divisor,
    visible: true,
    priority: "normal",
  };
}
