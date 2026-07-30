import { Dimension, getClientArea, type IDimension, type IRectangle } from "../../base/browser/geometry.js";
import { SerializableGrid, type ISerializableGridView, type SerializedGridDescriptor } from "../../base/browser/ui/grid/grid.js";
import { type Event, Emitter } from "../../base/common/event.js";
import { DisposableOwner } from "../../base/common/lifecycle.js";
import { type IWorkbenchLayoutService, type WorkbenchPartId, type WorkbenchPartVisibilityChangeEvent, workbenchPartIds } from "../services/layout/browser/layoutService.js";
import { type WorkbenchPart } from "./part.js";

const DEFAULT_LAYOUT_WIDTH = 1_024;
const DEFAULT_LAYOUT_HEIGHT = 768;
const DEFAULT_SIDEBAR_WIDTH = 220;
const DEFAULT_AUXILIARYBAR_WIDTH = 260;
const DEFAULT_PANEL_HEIGHT = 200;
const EDITOR_LAYOUT_PRIORITY = "high" as const;

export interface WorkbenchLayoutOptions {
  readonly initialDimension?: IDimension;
}

/**
 * The stable, mutable portion of Workbench layout state.
 *
 * The concrete layout owns this shape. The Layout Service exposes runtime
 * operations and does not make persisted representation part of its contract.
 */
export interface WorkbenchLayoutState {
  readonly version: 2;
  readonly sidebar: {
    readonly width: number;
    readonly visible: boolean;
  };
  readonly auxiliarybar: {
    readonly width: number;
    readonly visible: boolean;
  };
  readonly panel: {
    readonly height: number;
    readonly visible: boolean;
  };
}

/**
 * Owns the Workbench's fixed topology and mutable pixel layout state.
 *
 * Parts remain mounted while hidden, allowing Grid to restore their last
 * visible size without reconstructing UI state.
 */
export class WorkbenchLayout
  extends DisposableOwner
  implements IWorkbenchLayoutService {
  private readonly views = new Map<WorkbenchPartId, WorkbenchPartView>();
  private readonly grid: SerializableGrid<WorkbenchPartView>;
  private readonly partVisibility = new Map<WorkbenchPartId, boolean>();
  private readonly _onDidChangePartVisibility = this.own(new Emitter<WorkbenchPartVisibilityChangeEvent>());

  readonly onDidChangePartVisibility = this._onDidChangePartVisibility.event;
  readonly element: HTMLDivElement;

  constructor(
    private readonly container: Element,
    parts: ReadonlyMap<WorkbenchPartId, WorkbenchPart>,
    options: WorkbenchLayoutOptions = {},
  ) {
    super();
    validateParts(parts);
    this.element = container.ownerDocument.createElement("div");
    this.element.className = "zeta-workbench-layout";
    container.append(this.element);
    this.defer(() => this.element.remove());

    for (const partId of workbenchPartIds) {
      this.views.set(
        partId,
        new WorkbenchPartView(partId, requiredPart(parts, partId)),
      );
    }
    const initialDimension = resolveInitialDimension(this.element, options);
    const initialState = createDefaultWorkbenchLayoutState();
    this.grid = this.own(SerializableGrid.deserialize(
      createWorkbenchGridDescriptor(this.views, initialDimension, initialState),
      { fromJSON: (data) => this.view(parseWorkbenchPartId(data)) },
      container.ownerDocument,
    ));
    this.element.append(this.grid.element);

    const ResizeObserverConstructor =
      container.ownerDocument.defaultView?.ResizeObserver;
    if (ResizeObserverConstructor) {
      const observer = new ResizeObserverConstructor(([entry]) => {
        if (!entry) return;
        const borderBox = entry.borderBoxSize[0];
        this.layout(new Dimension(
          borderBox?.inlineSize ?? entry.contentRect.width,
          borderBox?.blockSize ?? entry.contentRect.height,
        ));
      });
      observer.observe(this.element, { box: "border-box" });
      this.defer(() => observer.disconnect());
    }
  }

  layout(dimension: IDimension = getClientArea(this.element)): void {
    assertDimension(dimension);
    this.grid.layout(dimension.width, dimension.height);
    this.publishPartVisibility();
  }

  get state(): WorkbenchLayoutState {
    const sidebar = this.getPartSize("sidebar");
    const auxiliarybar = this.getPartSize("auxiliarybar");
    const panel = this.getPartSize("panel");
    return {
      version: 2,
      sidebar: {
        width: sidebar.width,
        visible: this.isPartVisible("sidebar"),
      },
      auxiliarybar: {
        width: auxiliarybar.width,
        visible: this.isPartVisible("auxiliarybar"),
      },
      panel: {
        height: panel.height,
        visible: this.isPartVisible("panel"),
      },
    };
  }

  restoreState(value: unknown): void {
    const state = parseWorkbenchLayoutState(value);
    this.resizePart("sidebar", this.getPartSize("sidebar").with(state.sidebar.width));
    this.resizePart("auxiliarybar", this.getPartSize("auxiliarybar").with(state.auxiliarybar.width));
    this.resizePart("panel", new Dimension(this.getPartSize("panel").width, state.panel.height));
    this.updatePartsVisibility(["sidebar"], state.sidebar.visible);
    this.updatePartsVisibility(["auxiliarybar"], state.auxiliarybar.visible);
    this.updatePartsVisibility(["panel"], state.panel.visible);
  }

  isPartVisible(partId: WorkbenchPartId): boolean {
    return this.grid.isViewVisible(this.view(partId));
  }

  showPart(partId: WorkbenchPartId): void {
    this.showParts([partId]);
  }

  showParts(partIds: readonly WorkbenchPartId[]): void {
    this.updatePartsVisibility(partIds, true);
  }

  hidePart(partId: WorkbenchPartId): void {
    this.hideParts([partId]);
  }

  hideParts(partIds: readonly WorkbenchPartId[]): void {
    this.updatePartsVisibility(partIds, false);
  }

  getPartSize(partId: WorkbenchPartId): Dimension {
    const size = this.grid.getViewSize(this.view(partId));
    return new Dimension(size.width, size.height);
  }

  resizePart(partId: WorkbenchPartId, dimension: IDimension): void {
    assertDimension(dimension);
    this.grid.resizeView(this.view(partId), dimension);
  }

  private updatePartsVisibility(
    partIds: readonly WorkbenchPartId[],
    visible: boolean,
  ): void {
    const uniquePartIds = [...new Set(partIds)];
    for (const partId of uniquePartIds) this.view(partId);
    const changed = uniquePartIds.filter(
      (partId) => this.isPartVisible(partId) !== visible,
    );
    for (const partId of changed) {
      this.grid.setViewVisible(this.view(partId), visible);
    }
    this.publishPartVisibility();
  }

  private publishPartVisibility(): void {
    for (const partId of workbenchPartIds) {
      const visible = this.isPartVisible(partId);
      if (this.partVisibility.get(partId) === visible) continue;
      this.partVisibility.set(partId, visible);
      this._onDidChangePartVisibility.fire({ partId, visible });
    }
  }

  private view(partId: WorkbenchPartId): WorkbenchPartView {
    const view = this.views.get(partId);
    if (!view) throw new Error(`Unknown Workbench Part: ${partId}`);
    return view;
  }
}

class WorkbenchPartView implements ISerializableGridView {
  constructor(
    readonly partId: WorkbenchPartId,
    readonly part: WorkbenchPart,
  ) {}

  get element(): HTMLElement { return this.part.element; }
  get minimumWidth(): number { return this.part.minimumWidth; }
  get maximumWidth(): number { return this.part.maximumWidth; }
  get minimumHeight(): number { return this.part.minimumHeight; }
  get maximumHeight(): number { return this.part.maximumHeight; }
  get onDidChange(): Event<void> { return this.part.onDidChangeConstraints; }

  layout(bounds: IRectangle): void {
    this.part.layout(new Dimension(bounds.width, bounds.height));
  }

  setVisible(visible: boolean): void {
    this.part.setVisible(visible);
  }

  toJSON(): WorkbenchPartId {
    return this.partId;
  }
}

function createWorkbenchGridDescriptor(
  views: ReadonlyMap<WorkbenchPartId, WorkbenchPartView>,
  dimension: IDimension,
  state: WorkbenchLayoutState,
): SerializedGridDescriptor {
  const leaf = (
    partId: WorkbenchPartId,
    size: number,
    visible = true,
    priority: "normal" | "high" = "normal",
  ): SerializedGridDescriptor => ({
    type: "leaf",
    data: partId,
    size,
    visible,
    priority,
  });
  const titlebarHeight = requiredView(views, "titlebar").minimumHeight;
  const statusbarHeight = requiredView(views, "statusbar").minimumHeight;
  const bodyHeight = Math.max(
    0,
    dimension.height - titlebarHeight - statusbarHeight,
  );
  const panelHeight = state.panel.height;
  const editorHeight = Math.max(
    0,
    bodyHeight - (state.panel.visible ? panelHeight : 0),
  );
  const editorWidth = Math.max(
    0,
    dimension.width -
      (state.sidebar.visible ? state.sidebar.width : 0) -
      (state.auxiliarybar.visible ? state.auxiliarybar.width : 0),
  );
  return {
    type: "branch",
    orientation: "vertical",
    size: dimension.height,
    priority: "normal",
    children: [
      leaf("titlebar", titlebarHeight),
      {
        type: "branch",
        orientation: "horizontal",
        size: bodyHeight,
        priority: EDITOR_LAYOUT_PRIORITY,
        children: [
          leaf("sidebar", state.sidebar.width, state.sidebar.visible),
          {
            type: "branch",
            orientation: "vertical",
            size: editorWidth,
            priority: EDITOR_LAYOUT_PRIORITY,
            children: [
              leaf("editor", editorHeight, true, EDITOR_LAYOUT_PRIORITY),
              leaf("panel", panelHeight, state.panel.visible),
            ],
          },
          leaf(
            "auxiliarybar",
            state.auxiliarybar.width,
            state.auxiliarybar.visible,
          ),
        ],
      },
      leaf("statusbar", statusbarHeight),
    ],
  };
}

function createDefaultWorkbenchLayoutState(): WorkbenchLayoutState {
  return {
    version: 2,
    sidebar: {
      width: DEFAULT_SIDEBAR_WIDTH,
      visible: true,
    },
    auxiliarybar: {
      width: DEFAULT_AUXILIARYBAR_WIDTH,
      visible: true,
    },
    panel: {
      height: DEFAULT_PANEL_HEIGHT,
      visible: true,
    },
  };
}

function resolveInitialDimension(
  container: HTMLElement,
  options: WorkbenchLayoutOptions,
): Dimension {
  if (options.initialDimension) {
    assertDimension(options.initialDimension);
    return new Dimension(
      options.initialDimension.width,
      options.initialDimension.height,
    );
  }
  const measured = getClientArea(container);
  return new Dimension(
    measured.width > 0 ? measured.width : DEFAULT_LAYOUT_WIDTH,
    measured.height > 0 ? measured.height : DEFAULT_LAYOUT_HEIGHT,
  );
}

function validateParts(
  parts: ReadonlyMap<WorkbenchPartId, WorkbenchPart>,
): void {
  const missing = workbenchPartIds.filter((partId) => !parts.has(partId));
  if (missing.length > 0) {
    throw new TypeError(
      `Workbench layout is missing Parts: ${missing.join(", ")}`,
    );
  }
}

function requiredPart(
  parts: ReadonlyMap<WorkbenchPartId, WorkbenchPart>,
  partId: WorkbenchPartId,
): WorkbenchPart {
  const part = parts.get(partId);
  if (!part) throw new Error(`Workbench Part is not registered: ${partId}`);
  return part;
}

function requiredView(
  views: ReadonlyMap<WorkbenchPartId, WorkbenchPartView>,
  partId: WorkbenchPartId,
): WorkbenchPartView {
  const view = views.get(partId);
  if (!view) throw new Error(`Workbench Part view is not registered: ${partId}`);
  return view;
}

function parseWorkbenchPartId(value: unknown): WorkbenchPartId {
  if (typeof value === "string" && workbenchPartIds.includes(value as WorkbenchPartId)) {
    return value as WorkbenchPartId;
  }
  throw new TypeError("Workbench Grid contains an unknown Part");
}

function assertDimension(dimension: IDimension): void {
  if (
    !Number.isFinite(dimension.width) ||
    dimension.width < 0 ||
    !Number.isFinite(dimension.height) ||
    dimension.height < 0
  ) {
    throw new RangeError(
      "Workbench layout dimensions must be non-negative and finite",
    );
  }
}

function parseWorkbenchLayoutState(value: unknown): WorkbenchLayoutState {
  if (!isRecord(value) || !isHorizontalLayoutRegionState(value.sidebar) || !isHorizontalLayoutRegionState(value.auxiliarybar)) {
    throw new TypeError("Workbench layout state is invalid or unsupported");
  }
  let panel: { readonly height: number; readonly visible: boolean };
  if (value.version === 1) {
    panel = { height: 200, visible: true };
  } else if (value.version === 2 && isVerticalLayoutRegionState(value.panel)) {
    panel = value.panel;
  } else {
    throw new TypeError("Workbench layout state is invalid or unsupported");
  }
  return {
    version: 2,
    sidebar: {
      width: value.sidebar.width,
      visible: value.sidebar.visible,
    },
    auxiliarybar: {
      width: value.auxiliarybar.width,
      visible: value.auxiliarybar.visible,
    },
    panel: {
      height: panel.height,
      visible: panel.visible,
    },
  };
}

function isHorizontalLayoutRegionState(value: unknown): value is { readonly width: number; readonly visible: boolean } {
  return isRecord(value) && isLayoutDimension(value.width) && typeof value.visible === "boolean";
}

function isVerticalLayoutRegionState(value: unknown): value is { readonly height: number; readonly visible: boolean } {
  return isRecord(value) && isLayoutDimension(value.height) && typeof value.visible === "boolean";
}

function isLayoutDimension(value: unknown): value is number {
  return typeof value === "number" && Number.isFinite(value) && value >= 0;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}
