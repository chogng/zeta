import { Emitter } from "../../../common/event.js";
import { DisposableOwner } from "../../../common/lifecycle.js";
import { SplitView, } from "../splitview/splitview.js";
class GridNode {
    parent;
    width = 0;
    height = 0;
    top = 0;
    left = 0;
    priority = "normal";
}
class LeafNode extends GridNode {
    view;
    visible;
    constructor(view, visible) {
        super();
        this.view = view;
        this.visible = visible;
    }
    get element() { return this.view.element; }
    get minimumWidth() { return this.view.minimumWidth; }
    get maximumWidth() { return this.view.maximumWidth; }
    get minimumHeight() { return this.view.minimumHeight; }
    get maximumHeight() { return this.view.maximumHeight; }
    isVisible() { return this.visible; }
    setDisplayed(visible) {
        this.element.hidden = !visible;
        this.view.setVisible?.(visible);
    }
    layout(width, height, top, left) {
        this.width = width;
        this.height = height;
        this.top = top;
        this.left = left;
        this.view.layout({ width, height, top, left });
    }
}
class BranchNode extends GridNode {
    orientation;
    element;
    children;
    splitView;
    constructor(orientation, descriptors, priority, host) {
        super();
        this.orientation = orientation;
        this.priority = priority;
        if (descriptors.length === 0) {
            throw new TypeError("Grid branches must contain at least one child");
        }
        this.splitView = host.ownSplitView(new SplitView(orientation, host.ownerDocument));
        this.element = this.splitView.element;
        this.children = descriptors.map((descriptor) => host.createNode(descriptor));
        for (const [index, child] of this.children.entries()) {
            child.parent = { branch: this, index };
            this.splitView.addView(new AxisView(child, orientation), descriptorSizing(descriptors[index]));
        }
        host.ownEvent(this.splitView.onDidChangeViewSizes(() => {
            host.handleSplitViewChange();
        }));
    }
    get minimumWidth() {
        return this.#axisConstraint("minimumWidth", Math.max, 0);
    }
    get maximumWidth() {
        return this.#axisConstraint("maximumWidth", Math.min, Number.POSITIVE_INFINITY);
    }
    get minimumHeight() {
        return this.#axisConstraint("minimumHeight", Math.max, 0);
    }
    get maximumHeight() {
        return this.#axisConstraint("maximumHeight", Math.min, Number.POSITIVE_INFINITY);
    }
    isVisible() {
        return this.children.some((child) => child.isVisible());
    }
    setDisplayed(visible) {
        this.element.hidden = !visible;
    }
    layout(width, height, top, left) {
        this.width = width;
        this.height = height;
        this.top = top;
        this.left = left;
        if (this.orientation === "horizontal") {
            this.splitView.layout(width, height);
        }
        else {
            this.splitView.layout(height, width);
        }
    }
    #axisConstraint(property, orthogonalReducer, orthogonalInitial) {
        const visible = this.children.filter((child) => child.isVisible());
        if (visible.length === 0)
            return 0;
        const isPrimary = this.orientation === "horizontal"
            ? property.endsWith("Width")
            : property.endsWith("Height");
        if (isPrimary) {
            return visible.reduce((total, child) => total + child[property], 0);
        }
        return visible.reduce((result, child) => orthogonalReducer(result, child[property]), orthogonalInitial);
    }
}
class AxisView {
    node;
    orientation;
    constructor(node, orientation) {
        this.node = node;
        this.orientation = orientation;
    }
    get element() { return this.node.element; }
    get priority() { return this.node.priority; }
    get minimumSize() {
        return this.orientation === "horizontal"
            ? this.node.minimumWidth
            : this.node.minimumHeight;
    }
    get maximumSize() {
        return this.orientation === "horizontal"
            ? this.node.maximumWidth
            : this.node.maximumHeight;
    }
    layout(size, offset, orthogonalSize) {
        const parent = this.node.parent?.branch;
        const parentTop = parent?.top ?? 0;
        const parentLeft = parent?.left ?? 0;
        if (this.orientation === "horizontal") {
            this.node.layout(size, orthogonalSize, parentTop, parentLeft + offset);
        }
        else {
            this.node.layout(orthogonalSize, size, parentTop + offset, parentLeft);
        }
    }
    setVisible(visible) {
        this.node.setDisplayed(visible);
    }
}
/**
 * A two-dimensional layout implemented as a nested tree of SplitViews.
 *
 * The descriptor is structural input only. Runtime sizes and visibility are
 * owned by the SplitViews so callers do not need to rebuild the tree.
 */
export class Grid extends DisposableOwner {
    element;
    #root;
    #leaves = new Map();
    #onDidChange = this.own(new Emitter());
    #layoutWidth = 0;
    #layoutHeight = 0;
    #didLayout = false;
    #layingOut = false;
    onDidChange = this.#onDidChange.event;
    constructor(descriptor, ownerDocument = document) {
        super();
        validateDescriptor(descriptor, new Set());
        this.element = ownerDocument.createElement("div");
        this.element.className = "zeta-grid";
        this.defer(() => this.element.remove());
        const host = {
            ownerDocument,
            ownSplitView: (splitView) => this.own(splitView),
            ownEvent: (disposable) => {
                this.own(disposable);
            },
            createNode: (nodeDescriptor) => this.#createNode(nodeDescriptor, host),
            handleSplitViewChange: () => {
                if (!this.#layingOut)
                    this.#onDidChange.fire();
            },
        };
        this.#root = this.#createNode(descriptor, host);
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
    layout(width, height) {
        assertDimension(width, "width");
        assertDimension(height, "height");
        this.#layoutWidth = width;
        this.#layoutHeight = height;
        this.#didLayout = true;
        this.#layingOut = true;
        try {
            this.#root.layout(width, height, 0, 0);
        }
        finally {
            this.#layingOut = false;
        }
    }
    getViewSize(view) {
        const leaf = this.#leaf(view);
        return { width: leaf.width, height: leaf.height };
    }
    resizeView(view, dimension) {
        assertDimension(dimension.width, "view width");
        assertDimension(dimension.height, "view height");
        const leaf = this.#leaf(view);
        this.#resizeOnAxis(leaf, "horizontal", dimension.width);
        this.#resizeOnAxis(leaf, "vertical", dimension.height);
    }
    isViewVisible(view) {
        return this.#leaf(view).visible;
    }
    setViewVisible(view, visible) {
        const leaf = this.#leaf(view);
        if (leaf.visible === visible)
            return;
        leaf.visible = visible;
        leaf.setDisplayed(visible);
        let node = leaf;
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
    #createNode(descriptor, host) {
        if (descriptor.type === "leaf") {
            const leaf = new LeafNode(descriptor.view, descriptor.visible !== false);
            leaf.priority = descriptor.priority ?? "normal";
            if (this.#leaves.has(descriptor.view)) {
                throw new Error("Grid cannot contain the same view twice");
            }
            this.#leaves.set(descriptor.view, leaf);
            return leaf;
        }
        return new BranchNode(descriptor.orientation, descriptor.children, descriptor.priority ?? "normal", host);
    }
    #resizeOnAxis(leaf, orientation, size) {
        let node = leaf;
        while (node.parent) {
            const { branch, index } = node.parent;
            if (branch.orientation === orientation) {
                branch.splitView.resizeView(index, size);
                return;
            }
            node = branch;
        }
    }
    #leaf(view) {
        const leaf = this.#leaves.get(view);
        if (!leaf)
            throw new Error("Grid view is not registered");
        return leaf;
    }
}
function descriptorSizing(descriptor) {
    return descriptor.type === "leaf" && descriptor.visible === false
        ? { type: "invisible", cachedVisibleSize: descriptor.size }
        : descriptor.size;
}
function validateDescriptor(descriptor, seenViews) {
    assertDimension(descriptor.size, "descriptor size");
    if (descriptor.type === "leaf") {
        if (seenViews.has(descriptor.view)) {
            throw new Error("Grid cannot contain the same view twice");
        }
        seenViews.add(descriptor.view);
        validateViewConstraints(descriptor.view);
        return;
    }
    if (descriptor.orientation !== "horizontal" &&
        descriptor.orientation !== "vertical") {
        throw new TypeError("Grid branch orientation is invalid");
    }
    if (descriptor.children.length === 0) {
        throw new TypeError("Grid branches must contain at least one child");
    }
    for (const child of descriptor.children) {
        validateDescriptor(child, seenViews);
    }
}
function assertDimension(value, name) {
    if (!Number.isFinite(value) || value < 0) {
        throw new RangeError(`Grid ${name} must be a non-negative finite number`);
    }
}
function validateViewConstraints(view) {
    for (const [minimum, maximum, axis] of [
        [view.minimumWidth, view.maximumWidth, "width"],
        [view.minimumHeight, view.maximumHeight, "height"],
    ]) {
        assertDimension(minimum, `view minimum ${axis}`);
        if (typeof maximum !== "number" ||
            Number.isNaN(maximum) ||
            maximum < minimum) {
            throw new RangeError(`Grid view maximum ${axis} must be at least its minimum ${axis}`);
        }
    }
}
