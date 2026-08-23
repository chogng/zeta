import type { IDimension } from "../../../../base/browser/geometry.js";
import type { Event } from "../../../../base/common/event.js";
import { createServiceIdentifier } from "../../../../platform/instantiation/common/instantiation.js";

export const workbenchPartIds = ["titlebar", "statusbar", "sidebar", "auxiliarybar", "agentSidebar", "editor", "panel"] as const;

export type WorkbenchPartId = typeof workbenchPartIds[number];

/** Describes a Workbench Part whose effective visibility changed. */
export interface WorkbenchPartVisibilityChangeEvent {
  readonly partId: WorkbenchPartId;
  readonly visible: boolean;
}

/** Window-scoped Part operations implemented by the Workbench layout owner. */
export interface IWorkbenchLayoutService {
  readonly onDidChangePartVisibility: Event<WorkbenchPartVisibilityChangeEvent>;
  isPartVisible(partId: WorkbenchPartId): boolean;
  showPart(partId: WorkbenchPartId): void;
  showParts(partIds: readonly WorkbenchPartId[]): void;
  hidePart(partId: WorkbenchPartId): void;
  hideParts(partIds: readonly WorkbenchPartId[]): void;
  getPartSize(partId: WorkbenchPartId): IDimension;
  resizePart(partId: WorkbenchPartId, dimension: IDimension): void;
}

export const IWorkbenchLayoutService = createServiceIdentifier<IWorkbenchLayoutService>("workbenchLayoutService");
