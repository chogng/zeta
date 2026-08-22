import { createServiceIdentifier } from "../../instantiation/common/instantiation.js";

export type WorkspaceTrustSetting = "restricted" | "trusted";
export type WorkspaceTrustState = "restricted" | "trusted";

export interface WorkspaceTrustEntry {
  readonly workspace: string;
  readonly root: string | undefined;
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

/** Frontend-owned contract for managing the durable trusted-folder allowlist. */
export interface IWorkspaceTrustService {
  /** Lists trusted folders only; an absent entry remains the normal Restricted-mode state. */
  list(): Promise<WorkspaceTrustSnapshot>;
  /** Reads the effective state for one exact filesystem root. */
  read(root: string): Promise<WorkspaceTrustState>;
  /** Adds a trusted root; the compatibility Restricted value removes its durable entry. */
  set(root: string, setting: WorkspaceTrustSetting, expectedRevision: number): Promise<WorkspaceTrustCommandResult>;
  forget(workspace: string, expectedRevision: number): Promise<WorkspaceTrustCommandResult>;
}

export const IWorkspaceTrustService = createServiceIdentifier<IWorkspaceTrustService>("workspaceTrustService");
