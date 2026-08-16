import { Dimension, getClientArea, type IDimension } from "../../../base/browser/geometry.js";
import { SerializableGrid } from "../../../base/browser/ui/grid/grid.js";
import type { IResizable } from "../../../base/browser/ui/resizable/resizable.js";
import { type Event, Emitter } from "../../../base/common/event.js";
import { DisposableOwner } from "../../../base/common/lifecycle.js";
import type { ILayoutOffsetInfo } from "../../../platform/layout/common/layoutService.js";
import type { IStorageService } from "../../../platform/storage/common/storage.js";
import { type IWorkbenchLayoutService, type WorkbenchPartId, type WorkbenchPartVisibilityChangeEvent, workbenchPartIds } from "../../services/layout/common/workbenchLayoutService.js";
import type { WorkbenchPart } from "../part.js";
import type { WorkbenchSession } from "../workbenchSession.js";
import { createDefaultWorkbenchLayoutState, parseWorkbenchLayoutState, type WorkbenchLayoutState, WorkbenchLayoutStateModel } from "./workbenchLayoutState.js";
import { WorkbenchPartView } from "./workbenchPartView.js";
import { assertDimension, createWorkbenchGridDescriptor, parseWorkbenchPartId, resolveInitialDimension } from "./workbenchLayoutTopology.js";

const WINDOW_LEFT_EDGE_INSET = 6;
const WINDOW_RIGHT_EDGE_INSET = 8;
const PART_GUTTER_HALF = 3;
const PART_GUTTER_SIZE = PART_GUTTER_HALF * 2;

export interface WorkbenchLayoutOptions {
  readonly initialDimension?: IDimension;
  readonly session?: WorkbenchSession;
  readonly storageService?: IStorageService;
}

/**
 * Owns the Workbench's fixed Part topology and mutable pixel layout state.
 *
 * Container geometry is supplied through the generic `IResizable` contract; this
 * class only translates those dimensions into Grid bounds and Part layout calls.
 */
export class WorkbenchLayout
  extends DisposableOwner
  implements IResizable, IWorkbenchLayoutService {
  private readonly views = new Map<WorkbenchPartId, WorkbenchPartView>();
  private readonly grid: SerializableGrid<WorkbenchPartView>;
  private readonly stateModel: WorkbenchLayoutStateModel;
  private readonly partVisibility = new Map<WorkbenchPartId, boolean>();
  private readonly _onDidChangePartVisibility = this.own(
    new Emitter<WorkbenchPartVisibilityChangeEvent>(),
  );

  readonly onDidChangePartVisibility = this._onDidChangePartVisibility.event;
  readonly element: HTMLDivElement;

  constructor(
    container: Element,
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
        new WorkbenchPartView(partId, requiredPart(parts, partId), {
          snap: isSnappableWorkbenchPart(partId),
        }),
      );
    }
    const initialDimension = resolveInitialDimension(
      this.element,
      options.initialDimension,
    );
    this.stateModel = new WorkbenchLayoutStateModel(
      options.storageService,
      options.session ? parseWorkbenchLayoutState(options.session.layout) : createDefaultWorkbenchLayoutState(),
    );
    const initialState = this.stateModel.state;
    this.projectPartFrameInsets(
      initialState.sidebar.visible,
      initialState.auxiliarybar.visible,
      initialState.agentSidebar.visible,
      initialState.panel.visible,
    );
    this.grid = this.own(SerializableGrid.deserialize(
      createWorkbenchGridDescriptor(this.views, initialDimension, initialState),
      { fromJSON: (data) => this.view(parseWorkbenchPartId(data)) },
      container.ownerDocument,
      {
        sashPresentation: { type: "inset", gap: PART_GUTTER_SIZE },
        edgeSnapping: true,
      },
    ));
    this.element.append(this.grid.element);
    this.own(this.grid.onDidChange(() => {
      this.projectPartFrameInsets();
      this.publishPartVisibility();
    }));
    if (options.storageService) {
      this.own(options.storageService.onWillSaveState(() => {
        this.saveState();
      }));
    }
  }

  /** Offset information consumed by the platform layout service for overlays. */
  get mainContainerOffset(): ILayoutOffsetInfo {
    const titlebar = this.getPartSize("titlebar");
    return {
      top: this.isPartVisible("titlebar") ? titlebar.height : 0,
      quickInputTop: 0,
    };
  }

  layout(dimension: IDimension = getClientArea(this.element)): void {
    assertDimension(dimension);
    this.projectPartFrameInsets();
    this.grid.layout(dimension.width, dimension.height);
    this.publishPartVisibility();
  }

  get state(): WorkbenchLayoutState {
    const sidebar = this.getPartSize("sidebar");
    const auxiliarybar = this.getPartSize("auxiliarybar");
    const agentSidebar = this.getPartSize("agentSidebar");
    const panel = this.getPartSize("panel");
    return {
      version: 3,
      sidebar: {
        width: sidebar.width,
        visible: this.isPartVisible("sidebar"),
      },
      auxiliarybar: {
        width: auxiliarybar.width,
        visible: this.isPartVisible("auxiliarybar"),
      },
      agentSidebar: {
        width: agentSidebar.width,
        visible: this.isPartVisible("agentSidebar"),
      },
      panel: {
        height: panel.height,
        visible: this.isPartVisible("panel"),
      },
    };
  }

  restoreState(value: unknown): void {
    const state = parseWorkbenchLayoutState(value);
    this.applyState(state);
    this.saveState();
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

  private saveState(): void {
    this.stateModel.save(this.state);
  }

  private applyState(state: WorkbenchLayoutState): void {
    this.resizePart("sidebar", this.getPartSize("sidebar").with(state.sidebar.width));
    this.resizePart("auxiliarybar", this.getPartSize("auxiliarybar").with(state.auxiliarybar.width));
    this.resizePart("agentSidebar", this.getPartSize("agentSidebar").with(state.agentSidebar.width));
    this.resizePart("panel", new Dimension(this.getPartSize("panel").width, state.panel.height));
    this.updatePartsVisibility(["sidebar"], state.sidebar.visible);
    this.updatePartsVisibility(["auxiliarybar"], state.auxiliarybar.visible);
    this.updatePartsVisibility(["agentSidebar"], state.agentSidebar.visible);
    this.updatePartsVisibility(["panel"], state.panel.visible);
  }

  private updatePartsVisibility(
    partIds: readonly WorkbenchPartId[],
    visible: boolean,
  ): void {
    const uniquePartIds = [...new Set(partIds)];
    for (const partId of uniquePartIds) this.view(partId);
    this.projectPartFrameInsets(
      uniquePartIds.includes("sidebar") ? visible : this.isPartVisible("sidebar"),
      uniquePartIds.includes("auxiliarybar") ? visible : this.isPartVisible("auxiliarybar"),
      uniquePartIds.includes("agentSidebar") ? visible : this.isPartVisible("agentSidebar"),
      uniquePartIds.includes("panel") ? visible : this.isPartVisible("panel"),
    );
    const changed = uniquePartIds.filter(
      (partId) => this.isPartVisible(partId) !== visible,
    );
    for (const partId of changed) {
      this.grid.setViewVisible(this.view(partId), visible);
    }
    this.publishPartVisibility();
  }

  private projectPartFrameInsets(
    sidebarVisible = this.isPartVisible("sidebar"),
    auxiliarybarVisible = this.isPartVisible("auxiliarybar"),
    agentSidebarVisible = this.isPartVisible("agentSidebar"),
    panelVisible = this.isPartVisible("panel"),
  ): void {
    const centralInsets = {
      left: sidebarVisible ? PART_GUTTER_HALF : WINDOW_LEFT_EDGE_INSET,
      right: auxiliarybarVisible || agentSidebarVisible
        ? PART_GUTTER_HALF
        : WINDOW_RIGHT_EDGE_INSET,
    };
    this.view("sidebar").setFrameInsets({
      top: 0,
      right: PART_GUTTER_HALF,
      bottom: 0,
      left: WINDOW_LEFT_EDGE_INSET,
    });
    this.view("auxiliarybar").setFrameInsets({
      top: 0,
      right: agentSidebarVisible ? PART_GUTTER_HALF : WINDOW_RIGHT_EDGE_INSET,
      bottom: 0,
      left: PART_GUTTER_HALF,
    });
    this.view("agentSidebar").setFrameInsets({
      top: 0,
      right: WINDOW_RIGHT_EDGE_INSET,
      bottom: 0,
      left: PART_GUTTER_HALF,
    });
    this.view("editor").setFrameInsets({
      top: 0,
      right: centralInsets.right,
      bottom: panelVisible ? PART_GUTTER_HALF : 0,
      left: centralInsets.left,
    });
    this.view("panel").setFrameInsets({
      top: panelVisible ? PART_GUTTER_HALF : 0,
      right: centralInsets.right,
      bottom: 0,
      left: centralInsets.left,
    });
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

function isSnappableWorkbenchPart(partId: WorkbenchPartId): boolean {
  return partId === "sidebar" ||
    partId === "auxiliarybar" ||
    partId === "agentSidebar" ||
    partId === "panel";
}
