import { Dimension, type IDimension } from "../../../base/browser/geometry.js";
import type { SerializedGridDescriptor } from "../../../base/browser/ui/grid/grid.js";
import { type WorkbenchPartId, workbenchPartIds } from "../../services/layout/common/workbenchLayoutService.js";
import type { WorkbenchLayoutState } from "./workbenchLayoutState.js";
import type { WorkbenchPartView } from "./workbenchPartView.js";

const EDITOR_LAYOUT_PRIORITY = "high" as const;
const DEFAULT_LAYOUT_WIDTH = 1_024;
const DEFAULT_LAYOUT_HEIGHT = 768;

export function createWorkbenchGridDescriptor(
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
      (state.auxiliarybar.visible ? state.auxiliarybar.width : 0) -
      (state.agentSidebar.visible ? state.agentSidebar.width : 0),
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
          leaf(
            "agentSidebar",
            state.agentSidebar.width,
            state.agentSidebar.visible,
          ),
        ],
      },
      leaf("statusbar", statusbarHeight),
    ],
  };
}

export function resolveInitialDimension(
  container: HTMLElement,
  dimension: IDimension | undefined,
): Dimension {
  if (dimension) {
    assertDimension(dimension);
    if (dimension.width > 0 && dimension.height > 0) {
      return new Dimension(dimension.width, dimension.height);
    }
  }
  const measured = {
    width: container.clientWidth,
    height: container.clientHeight,
  };
  return new Dimension(
    measured.width > 0 ? measured.width : DEFAULT_LAYOUT_WIDTH,
    measured.height > 0 ? measured.height : DEFAULT_LAYOUT_HEIGHT,
  );
}

export function parseWorkbenchPartId(value: unknown): WorkbenchPartId {
  if (
    typeof value === "string" &&
    workbenchPartIds.includes(value as WorkbenchPartId)
  ) {
    return value as WorkbenchPartId;
  }
  throw new TypeError("Workbench Grid contains an unknown Part");
}

export function requiredView(
  views: ReadonlyMap<WorkbenchPartId, WorkbenchPartView>,
  partId: WorkbenchPartId,
): WorkbenchPartView {
  const view = views.get(partId);
  if (!view) throw new Error(`Workbench Part view is not registered: ${partId}`);
  return view;
}

export function assertDimension(dimension: IDimension): void {
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
