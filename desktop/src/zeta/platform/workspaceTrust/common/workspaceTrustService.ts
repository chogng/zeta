import { createServiceIdentifier } from "../../instantiation/common/instantiation.js";

export type WorkspaceTrustSetting = "restricted" | "trusted";

export interface WorkspaceTrustEntry {
  readonly workspace: string;
  readonly root: string | undefined;
  readonly setting: WorkspaceTrustSetting;
}

export interface WorkspaceTrustSnapshot {
  readonly revision: number;
  readonly entries: readonly WorkspaceTrustEntry[];
}

export interface WorkspaceTrustCommandResult {
  readonly revision: number;
  readonly generation: number;
  readonly disposition: "updated" | "replayed";
}

/** Frontend-owned contract for listing and revoking durable User Workspace Trust decisions. */
export interface IWorkspaceTrustService {
  list(): Promise<WorkspaceTrustSnapshot>;
  set(root: string, setting: WorkspaceTrustSetting, expectedRevision: number): Promise<WorkspaceTrustCommandResult>;
  forget(workspace: string, expectedRevision: number): Promise<WorkspaceTrustCommandResult>;
}

export const IWorkspaceTrustService = createServiceIdentifier<IWorkspaceTrustService>("workspaceTrustService");
