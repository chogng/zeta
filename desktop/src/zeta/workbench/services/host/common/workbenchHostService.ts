import type { Event } from "../../../../base/common/event.js";
import { createServiceIdentifier } from "../../../../platform/instantiation/common/instantiation.js";

export interface WorkbenchHostError {
  readonly kind: "runtimeError" | "unhandledRejection";
  readonly message: string;
  readonly source: string | undefined;
}

export interface WorkbenchTextDownload {
  readonly fileName: string;
  readonly content: string;
  readonly mediaType: string;
}

/** Window-host operations available without exposing the Workbench DOM root. */
export interface IWorkbenchHostService {
  readonly onDidError: Event<WorkbenchHostError>;
  downloadText(download: WorkbenchTextDownload): void;
}

export const IWorkbenchHostService = createServiceIdentifier<IWorkbenchHostService>("workbenchHostService");
