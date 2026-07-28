import { Dimension, getClientArea, } from "../../base/browser/geometry.js";
import { Grid, } from "../../base/browser/ui/grid/grid.js";
import { Emitter } from "../../base/common/event.js";
import { DisposableOwner } from "../../base/common/lifecycle.js";
import { createServiceIdentifier, } from "../../platform/instantiation/common/instantiation.js";
import { AuxiliaryBarVisibleContext, EditorAreaVisibleContext, SideBarVisibleContext, } from "../common/contextkeys.js";
export const workbenchPartIds = [
    "titlebar",
    "statusbar",
    "sidebar",
    "session",
    "auxiliarybar",
    "editor",
];
/** Publishes one runtime Part visibility change to its context key. */
export function applyWorkbenchPartVisibilityContext(contextKeyService, partId, visible) {
    switch (partId) {
        case "sidebar":
            contextKeyService.setContext(SideBarVisibleContext.key, visible);
            break;
        case "auxiliarybar":
            contextKeyService.setContext(AuxiliaryBarVisibleContext.key, visible);
            break;
        case "editor":
            contextKeyService.setContext(EditorAreaVisibleContext.key, visible);
            break;
    }
}
export const IWorkbenchLayoutService = createServiceIdentifier("workbenchLayoutService");
/**
 * Owns the Workbench's fixed topology and mutable pixel layout state.
 *
 * Parts remain mounted while hidden, allowing Grid to restore their last
 * visible size without reconstructing UI state.
 */
export class WorkbenchLayout extends DisposableOwner {
    container;
    #views = new Map();
    #grid;
    #partVisibility = new Map();
    #onDidChangePartVisibility = this.own(new Emitter());
    onDidChangePartVisibility = this.#onDidChangePartVisibility.event;
    element;
    constructor(container, parts) {
        super();
        this.container = container;
        validateParts(parts);
        this.element = container.ownerDocument.createElement("div");
        this.element.className = "zeta-workbench-layout";
        container.append(this.element);
        this.defer(() => this.element.remove());
        for (const partId of workbenchPartIds) {
            this.#views.set(partId, new WorkbenchPartView(requiredPart(parts, partId)));
        }
        this.#grid = this.own(new Grid(createWorkbenchGridDescriptor(this.#views), container.ownerDocument));
        this.element.append(this.#grid.element);
        const ResizeObserverConstructor = container.ownerDocument.defaultView?.ResizeObserver;
        if (ResizeObserverConstructor) {
            const observer = new ResizeObserverConstructor(([entry]) => {
                if (!entry)
                    return;
                const borderBox = entry.borderBoxSize[0];
                this.layout(new Dimension(borderBox?.inlineSize ?? entry.contentRect.width, borderBox?.blockSize ?? entry.contentRect.height));
            });
            observer.observe(this.element, { box: "border-box" });
            this.defer(() => observer.disconnect());
        }
    }
    layout(dimension = getClientArea(this.element)) {
        assertDimension(dimension);
        this.#grid.layout(dimension.width, dimension.height);
        this.#publishPartVisibility();
    }
    get state() {
        const sidebar = this.getPartSize("sidebar");
        const auxiliarybar = this.getPartSize("auxiliarybar");
        return {
            version: 1,
            sidebar: {
                width: sidebar.width,
                visible: this.isPartVisible("sidebar"),
            },
            auxiliarybar: {
                width: auxiliarybar.width,
                visible: this.isPartVisible("auxiliarybar"),
            },
        };
    }
    restoreState(value) {
        const state = parseWorkbenchLayoutState(value);
        this.resizePart("sidebar", this.getPartSize("sidebar").with(state.sidebar.width));
        this.resizePart("auxiliarybar", this.getPartSize("auxiliarybar").with(state.auxiliarybar.width));
        this.updatePartsVisibility(["sidebar"], state.sidebar.visible);
        this.updatePartsVisibility(["auxiliarybar"], state.auxiliarybar.visible);
    }
    isPartVisible(partId) {
        return this.#grid.isViewVisible(this.#view(partId));
    }
    showPart(partId) {
        this.showParts([partId]);
    }
    showParts(partIds) {
        this.updatePartsVisibility(partIds, true);
    }
    hidePart(partId) {
        this.hideParts([partId]);
    }
    hideParts(partIds) {
        this.updatePartsVisibility(partIds, false);
    }
    getPartSize(partId) {
        const size = this.#grid.getViewSize(this.#view(partId));
        return new Dimension(size.width, size.height);
    }
    resizePart(partId, dimension) {
        assertDimension(dimension);
        this.#grid.resizeView(this.#view(partId), dimension);
    }
    updatePartsVisibility(partIds, visible) {
        const uniquePartIds = [...new Set(partIds)];
        for (const partId of uniquePartIds)
            this.#view(partId);
        const changed = uniquePartIds.filter((partId) => this.isPartVisible(partId) !== visible);
        for (const partId of changed) {
            this.#grid.setViewVisible(this.#view(partId), visible);
        }
        this.#publishPartVisibility();
    }
    #publishPartVisibility() {
        for (const partId of workbenchPartIds) {
            const visible = this.isPartVisible(partId);
            if (this.#partVisibility.get(partId) === visible)
                continue;
            this.#partVisibility.set(partId, visible);
            this.#onDidChangePartVisibility.fire({ partId, visible });
        }
    }
    #view(partId) {
        const view = this.#views.get(partId);
        if (!view)
            throw new Error(`Unknown Workbench Part: ${partId}`);
        return view;
    }
}
class WorkbenchPartView {
    part;
    constructor(part) {
        this.part = part;
    }
    get element() { return this.part.element; }
    get minimumWidth() { return this.part.minimumWidth; }
    get maximumWidth() { return this.part.maximumWidth; }
    get minimumHeight() { return this.part.minimumHeight; }
    get maximumHeight() { return this.part.maximumHeight; }
    get onDidChange() { return this.part.onDidChangeConstraints; }
    layout(bounds) {
        this.part.layout(new Dimension(bounds.width, bounds.height));
    }
    setVisible(visible) {
        this.part.setVisible(visible);
    }
}
function createWorkbenchGridDescriptor(views) {
    const leaf = (partId, size) => ({
        type: "leaf",
        view: requiredView(views, partId),
        size,
    });
    return {
        type: "branch",
        orientation: "vertical",
        size: 768,
        children: [
            leaf("titlebar", 35),
            {
                type: "branch",
                orientation: "horizontal",
                size: 710,
                priority: "high",
                children: [
                    leaf("sidebar", 220),
                    {
                        type: "branch",
                        orientation: "vertical",
                        size: 584,
                        priority: "high",
                        children: [
                            leaf("session", 36),
                            {
                                ...leaf("editor", 674),
                                priority: "high",
                            },
                        ],
                    },
                    leaf("auxiliarybar", 220),
                ],
            },
            leaf("statusbar", 23),
        ],
    };
}
function validateParts(parts) {
    const missing = workbenchPartIds.filter((partId) => !parts.has(partId));
    if (missing.length > 0) {
        throw new TypeError(`Workbench layout is missing Parts: ${missing.join(", ")}`);
    }
}
function requiredPart(parts, partId) {
    const part = parts.get(partId);
    if (!part)
        throw new Error(`Workbench Part is not registered: ${partId}`);
    return part;
}
function requiredView(views, partId) {
    const view = views.get(partId);
    if (!view)
        throw new Error(`Workbench Part view is not registered: ${partId}`);
    return view;
}
function parseWorkbenchLayoutState(value) {
    if (!isRecord(value) ||
        value.version !== 1 ||
        !isLayoutRegionState(value.sidebar) ||
        !isLayoutRegionState(value.auxiliarybar)) {
        throw new TypeError("Workbench layout state is invalid or unsupported");
    }
    return {
        version: 1,
        sidebar: {
            width: value.sidebar.width,
            visible: value.sidebar.visible,
        },
        auxiliarybar: {
            width: value.auxiliarybar.width,
            visible: value.auxiliarybar.visible,
        },
    };
}
function isLayoutRegionState(value) {
    return isRecord(value) &&
        typeof value.width === "number" &&
        Number.isFinite(value.width) &&
        value.width >= 0 &&
        typeof value.visible === "boolean";
}
function assertDimension(dimension) {
    if (!Number.isFinite(dimension.width) ||
        dimension.width < 0 ||
        !Number.isFinite(dimension.height) ||
        dimension.height < 0) {
        throw new RangeError("Workbench layout dimensions must be non-negative and finite");
    }
}
function isRecord(value) {
    return typeof value === "object" && value !== null;
}
