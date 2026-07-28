import { URI } from "../../../base/common/uri.js";
import {
  createServiceIdentifier,
} from "../../instantiation/common/instantiation.js";

/** Describes whether a workbench contains no project, one folder, or a workspace. */
export const enum WorkbenchState {
  EMPTY = 1,
  FOLDER,
  WORKSPACE,
}

export interface IEmptyWorkspaceContext {
  readonly state: WorkbenchState.EMPTY;
}

export interface IFolderWorkspaceContext {
  readonly state: WorkbenchState.FOLDER;
  readonly uri: string;
  readonly label: string;
}

export interface IWorkspaceFileContext {
  readonly state: WorkbenchState.WORKSPACE;
  readonly configUri: string;
  readonly label: string;
}

/**
 * Identifies the project, if any, hosted by one workbench window.
 *
 * File URIs are identities only. Filesystem access and workspace-boundary
 * authorization remain outside the renderer.
 */
export type IWorkspaceContext =
  | IEmptyWorkspaceContext
  | IFolderWorkspaceContext
  | IWorkspaceFileContext;

/** Read-only workspace identity available to workbench contributions. */
export interface IWorkspaceContextService {
  getWorkspace(): IWorkspaceContext;
  getWorkbenchState(): WorkbenchState;
}

export const IWorkspaceContextService =
  createServiceIdentifier<IWorkspaceContextService>(
    "workspaceContextService",
  );

export const EMPTY_WORKSPACE: IEmptyWorkspaceContext =
  Object.freeze({ state: WorkbenchState.EMPTY });

/** Validates a workspace identity received across a process boundary. */
export function parseWorkspaceContext(value: unknown): IWorkspaceContext {
  const record = exactRecord(value);
  switch (record.state) {
    case WorkbenchState.EMPTY:
      requireExactKeys(record, ["state"]);
      return EMPTY_WORKSPACE;
    case WorkbenchState.FOLDER:
      requireExactKeys(record, ["label", "state", "uri"]);
      return Object.freeze({
        state: WorkbenchState.FOLDER,
        uri: fileUri(record.uri, "workspace uri"),
        label: nonEmptyString(record.label, "workspace label"),
      });
    case WorkbenchState.WORKSPACE:
      requireExactKeys(record, ["configUri", "label", "state"]);
      return Object.freeze({
        state: WorkbenchState.WORKSPACE,
        configUri: fileUri(record.configUri, "workspace config uri"),
        label: nonEmptyString(record.label, "workspace label"),
      });
    default:
      throw new Error("workbench state is invalid");
  }
}

function exactRecord(value: unknown): Record<string, unknown> {
  if (typeof value !== "object" || value === null || Array.isArray(value)) {
    throw new Error("workspace context must be an object");
  }
  return value as Record<string, unknown>;
}

function requireExactKeys(
  value: Record<string, unknown>,
  expected: readonly string[],
): void {
  const actual = Object.keys(value).sort();
  if (
    actual.length !== expected.length ||
    actual.some((key, index) => key !== expected[index])
  ) {
    throw new Error(
      `workspace context must contain exactly: ${expected.join(", ")}`,
    );
  }
}

function fileUri(value: unknown, field: string): string {
  if (typeof value !== "string") {
    throw new Error(`${field} must be a string`);
  }
  const uri = URI.parse(value);
  if (uri.scheme !== "file") {
    throw new Error(`${field} must use the file scheme`);
  }
  if (uri.query || uri.fragment) {
    throw new Error(`${field} must not contain a query or fragment`);
  }
  return uri.toString();
}

function nonEmptyString(value: unknown, field: string): string {
  if (typeof value !== "string" || value.trim().length === 0) {
    throw new Error(`${field} must be a non-empty string`);
  }
  return value;
}
