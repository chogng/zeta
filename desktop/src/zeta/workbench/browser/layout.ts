import {
  Dimension,
  getClientArea,
  type IDimension,
  type IRectangle,
} from "../../base/browser/geometry.js";
import {
  Grid,
  type GridDescriptor,
  type IGridView,
} from "../../base/browser/ui/grid/grid.js";
import { type Event, Emitter } from "../../base/common/event.js";
import { DisposableOwner } from "../../base/common/lifecycle.js";
import type {
  IContextKeyService,
} from "../../platform/contextkey/common/contextkey.js";
import {
  createServiceIdentifier,
} from "../../platform/instantiation/common/instantiation.js";
import {
  AuxiliaryBarVisibleContext,
  EditorAreaVisibleContext,
  SideBarVisibleContext,
} from "../common/contextkeys.js";
import { type WorkbenchPart } from "./part.js";

export const workbenchPartIds = [
  "titlebar",
  "statusbar",
  "sidebar",
  "session",
  "auxiliarybar",
  "editor",
] as const;
export type WorkbenchPartId = typeof workbenchPartIds[number];

/** Publishes one runtime Part visibility change to its context key. */
export function applyWorkbenchPartVisibilityContext(
  contextKeyService: IContextKeyService,
  partId: WorkbenchPartId,
  visible: boolean,
): void {
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

/** A Workbench Part whose effective layout visibility changed. */
export interface WorkbenchPartVisibilityChangeEvent {
  readonly partId: WorkbenchPartId;
  readonly visible: boolean;
}

/**
 * The stable, mutable portion of Workbench layout state.
 *
 * Topology is intentionally absent: migrations only need to handle user-sized
 * regions and visibility, never an arbitrary external layout tree.
 */
export interface WorkbenchLayoutState {
  readonly version: 1;
  readonly sidebar: {
    readonly width: number;
    readonly visible: boolean;
  };
  readonly auxiliarybar: {
    readonly width: number;
    readonly visible: boolean;
  };
}

/** Runtime layout operations available to Workbench contributions. */
export interface IWorkbenchLayoutService {
  readonly onDidChangePartVisibility: Event<
    WorkbenchPartVisibilityChangeEvent
  >;
  isPartVisible(partId: WorkbenchPartId): boolean;
  showPart(partId: WorkbenchPartId): void;
  showParts(partIds: readonly WorkbenchPartId[]): void;
  hidePart(partId: WorkbenchPartId): void;
  hideParts(partIds: readonly WorkbenchPartId[]): void;
  getPartSize(partId: WorkbenchPartId): IDimension;
  resizePart(partId: WorkbenchPartId, dimension: IDimension): void;
}

export const IWorkbenchLayoutService =
  createServiceIdentifier<IWorkbenchLayoutService>("workbenchLayoutService");

/**
 * Owns the Workbench's fixed topology and mutable pixel layout state.
 *
 * Parts remain mounted while hidden, allowing Grid to restore their last
 * visible size without reconstructing UI state.
 */
export class WorkbenchLayout
  extends DisposableOwner
  implements IWorkbenchLayoutService {
  readonly #views = new Map<WorkbenchPartId, WorkbenchPartView>();
  readonly #grid: Grid<WorkbenchPartView>;
  readonly #partVisibility = new Map<WorkbenchPartId, boolean>();
  readonly #onDidChangePartVisibility =
    this.own(new Emitter<WorkbenchPartVisibilityChangeEvent>());

  readonly onDidChangePartVisibility: Event<
    WorkbenchPartVisibilityChangeEvent
  > = this.#onDidChangePartVisibility.event;
  readonly element: HTMLDivElement;

  constructor(
    private readonly container: Element,
    parts: ReadonlyMap<WorkbenchPartId, WorkbenchPart>,
  ) {
    super();
    validateParts(parts);
    this.element = container.ownerDocument.createElement("div");
    this.element.className = "zeta-workbench-layout";
    container.append(this.element);
    this.defer(() => this.element.remove());

    for (const partId of workbenchPartIds) {
      this.#views.set(
        partId,
        new WorkbenchPartView(requiredPart(parts, partId)),
      );
    }
    this.#grid = this.own(new Grid(
      createWorkbenchGridDescriptor(this.#views),
      container.ownerDocument,
    ));
    this.element.append(this.#grid.element);

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
    this.#grid.layout(dimension.width, dimension.height);
    this.#publishPartVisibility();
  }

  get state(): WorkbenchLayoutState {
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

  restoreState(value: unknown): void {
    const state = parseWorkbenchLayoutState(value);
    this.resizePart(
      "sidebar",
      this.getPartSize("sidebar").with(state.sidebar.width),
    );
    this.resizePart(
      "auxiliarybar",
      this.getPartSize("auxiliarybar").with(state.auxiliarybar.width),
    );
    this.updatePartsVisibility(
      ["sidebar"],
      state.sidebar.visible,
    );
    this.updatePartsVisibility(
      ["auxiliarybar"],
      state.auxiliarybar.visible,
    );
  }

  isPartVisible(partId: WorkbenchPartId): boolean {
    return this.#grid.isViewVisible(this.#view(partId));
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
    const size = this.#grid.getViewSize(this.#view(partId));
    return new Dimension(size.width, size.height);
  }

  resizePart(partId: WorkbenchPartId, dimension: IDimension): void {
    assertDimension(dimension);
    this.#grid.resizeView(this.#view(partId), dimension);
  }

  private updatePartsVisibility(
    partIds: readonly WorkbenchPartId[],
    visible: boolean,
  ): void {
    const uniquePartIds = [...new Set(partIds)];
    for (const partId of uniquePartIds) this.#view(partId);
    const changed = uniquePartIds.filter(
      (partId) => this.isPartVisible(partId) !== visible,
    );
    for (const partId of changed) {
      this.#grid.setViewVisible(this.#view(partId), visible);
    }
    this.#publishPartVisibility();
  }

  #publishPartVisibility(): void {
    for (const partId of workbenchPartIds) {
      const visible = this.isPartVisible(partId);
      if (this.#partVisibility.get(partId) === visible) continue;
      this.#partVisibility.set(partId, visible);
      this.#onDidChangePartVisibility.fire({ partId, visible });
    }
  }

  #view(partId: WorkbenchPartId): WorkbenchPartView {
    const view = this.#views.get(partId);
    if (!view) throw new Error(`Unknown Workbench Part: ${partId}`);
    return view;
  }
}

class WorkbenchPartView implements IGridView {
  constructor(readonly part: WorkbenchPart) {}

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
}

function createWorkbenchGridDescriptor(
  views: ReadonlyMap<WorkbenchPartId, WorkbenchPartView>,
): GridDescriptor<WorkbenchPartView> {
  const leaf = (
    partId: WorkbenchPartId,
    size: number,
  ): GridDescriptor<WorkbenchPartView> => ({
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

function parseWorkbenchLayoutState(value: unknown): WorkbenchLayoutState {
  if (
    !isRecord(value) ||
    value.version !== 1 ||
    !isLayoutRegionState(value.sidebar) ||
    !isLayoutRegionState(value.auxiliarybar)
  ) {
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

function isLayoutRegionState(
  value: unknown,
): value is { readonly width: number; readonly visible: boolean } {
  return isRecord(value) &&
    typeof value.width === "number" &&
    Number.isFinite(value.width) &&
    value.width >= 0 &&
    typeof value.visible === "boolean";
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

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}
