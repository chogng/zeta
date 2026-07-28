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

/** Identity shared by empty, single-folder, and multi-root workspaces. */
export interface IBaseWorkspaceIdentifier {
  readonly id: string;
}

/** Identifies an empty workbench window. */
export interface IEmptyWorkspaceIdentifier extends IBaseWorkspaceIdentifier {
}

/** Identifies a workbench opened on one folder. */
export interface ISingleFolderWorkspaceIdentifier
  extends IBaseWorkspaceIdentifier {
  readonly uri: URI;
}

/** Identifies a workbench opened from a multi-root workspace file. */
export interface IWorkspaceIdentifier extends IBaseWorkspaceIdentifier {
  readonly configPath: URI;
}

/** Identifies the workspace, folder, or empty context hosted by one window. */
export type IAnyWorkspaceIdentifier =
  | IWorkspaceIdentifier
  | ISingleFolderWorkspaceIdentifier
  | IEmptyWorkspaceIdentifier;

/** A folder belonging to the current resolved workspace. */
export interface IWorkspaceFolder {
  readonly uri: URI;
  readonly name: string;
  readonly index: number;
}

/**
 * The resolved workspace visible to workbench contributions.
 *
 * Resource URIs are identities only. Filesystem access and workspace-boundary
 * authorization remain outside the renderer.
 */
export interface IWorkspace {
  readonly id: string;
  readonly folders: readonly IWorkspaceFolder[];
  readonly configuration?: URI;
  readonly name?: string;
}

/** Read-only workspace identity available to workbench contributions. */
export interface IWorkspaceContextService {
  getWorkspace(): IWorkspace;
  getWorkbenchState(): WorkbenchState;
}

export const IWorkspaceContextService =
  createServiceIdentifier<IWorkspaceContextService>(
    "workspaceContextService",
  );

/** Fallback identity for a window whose durable empty-workspace ID is unknown. */
export const UNKNOWN_EMPTY_WINDOW_WORKSPACE: IEmptyWorkspaceIdentifier =
  Object.freeze({ id: "empty-window" });

/** Returns whether a value identifies one folder. */
export function isSingleFolderWorkspaceIdentifier(
  value: unknown,
): value is ISingleFolderWorkspaceIdentifier {
  const candidate = value as Partial<ISingleFolderWorkspaceIdentifier> | null;
  return isNonEmptyString(candidate?.id) && candidate?.uri instanceof URI;
}

/** Returns whether a value identifies a multi-root workspace file. */
export function isWorkspaceIdentifier(
  value: unknown,
): value is IWorkspaceIdentifier {
  const candidate = value as Partial<IWorkspaceIdentifier> | null;
  return isNonEmptyString(candidate?.id) &&
    candidate?.configPath instanceof URI;
}

/** Returns whether a value identifies an empty workbench. */
export function isEmptyWorkspaceIdentifier(
  value: unknown,
): value is IEmptyWorkspaceIdentifier {
  const candidate = value as Partial<IEmptyWorkspaceIdentifier> | null;
  return isNonEmptyString(candidate?.id) &&
    !isSingleFolderWorkspaceIdentifier(value) &&
    !isWorkspaceIdentifier(value);
}

/** Derives workbench state without storing a second workspace discriminator. */
export function workbenchStateFromWorkspaceIdentifier(
  workspace: IAnyWorkspaceIdentifier,
): WorkbenchState {
  if (isWorkspaceIdentifier(workspace)) {
    return WorkbenchState.WORKSPACE;
  }
  if (isSingleFolderWorkspaceIdentifier(workspace)) {
    return WorkbenchState.FOLDER;
  }
  return WorkbenchState.EMPTY;
}

/** Converts a workspace identity into an IPC-safe plain object. */
export function serializeWorkspaceIdentifier(
  workspace: IAnyWorkspaceIdentifier,
): unknown {
  if (isWorkspaceIdentifier(workspace)) {
    return {
      id: workspace.id,
      configPath: workspace.configPath.toString(),
    };
  }
  if (isSingleFolderWorkspaceIdentifier(workspace)) {
    return {
      id: workspace.id,
      uri: workspace.uri.toString(),
    };
  }
  return { id: workspace.id };
}

/** Validates and revives a workspace identity received over IPC. */
export function parseWorkspaceIdentifier(
  value: unknown,
): IAnyWorkspaceIdentifier {
  const record = exactRecord(value);
  const id = nonEmptyString(record.id, "workspace id");
  if ("configPath" in record) {
    requireExactKeys(record, ["configPath", "id"]);
    return Object.freeze({
      id,
      configPath: fileUri(record.configPath, "workspace config path"),
    });
  }
  if ("uri" in record) {
    requireExactKeys(record, ["id", "uri"]);
    return Object.freeze({
      id,
      uri: fileUri(record.uri, "workspace folder uri"),
    });
  }
  requireExactKeys(record, ["id"]);
  return id === UNKNOWN_EMPTY_WINDOW_WORKSPACE.id
    ? UNKNOWN_EMPTY_WINDOW_WORKSPACE
    : Object.freeze({ id });
}

function exactRecord(value: unknown): Record<string, unknown> {
  if (typeof value !== "object" || value === null || Array.isArray(value)) {
    throw new Error("workspace identifier must be an object");
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
      `workspace identifier must contain exactly: ${expected.join(", ")}`,
    );
  }
}

function fileUri(value: unknown, field: string): URI {
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
  return uri;
}

function nonEmptyString(value: unknown, field: string): string {
  if (!isNonEmptyString(value)) {
    throw new Error(`${field} must be a non-empty string`);
  }
  return value;
}

function isNonEmptyString(value: unknown): value is string {
  return typeof value === "string" && value.trim().length > 0;
}
