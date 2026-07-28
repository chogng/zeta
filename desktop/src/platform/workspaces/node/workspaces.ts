import { createHash } from "node:crypto";
import { realpath, stat } from "node:fs/promises";
import { extname, resolve } from "node:path";
import { URI } from "../../../base/common/uri.js";
import {
  type IAnyWorkspaceIdentifier,
  type ISingleFolderWorkspaceIdentifier,
  type IWorkspaceIdentifier,
  UNKNOWN_EMPTY_WINDOW_WORKSPACE,
} from "../../workspace/common/workspace.js";
import {
  type IWorkspaceOpenTarget,
  WorkspaceOpenTargetKind,
} from "../common/workspaces.js";

const ZETA_WORKSPACE_EXTENSION = ".zeta-workspace";

/** Filesystem shape found for a requested workspace path. */
export const enum WorkspacePathKind {
  Directory,
  File,
  Other,
}

/** Canonical filesystem result used while resolving an open target. */
export interface IResolvedWorkspacePath {
  readonly kind: WorkspacePathKind;
  readonly path: string;
}

/** Host filesystem operations needed to canonicalize a workspace path. */
export interface IWorkspacePathService {
  resolvePath(path: string): Promise<IResolvedWorkspacePath>;
}

/** Native path service used by the Electron main process. */
export const nodeWorkspacePathService: IWorkspacePathService = {
  async resolvePath(path): Promise<IResolvedWorkspacePath> {
    const canonicalPath = await realpath(path);
    const metadata = await stat(canonicalPath);
    const kind = metadata.isDirectory()
      ? WorkspacePathKind.Directory
      : metadata.isFile()
        ? WorkspacePathKind.File
        : WorkspacePathKind.Other;
    return { kind, path: canonicalPath };
  },
};

/**
 * Resolves one launch target into a stable folder or workspace identity.
 *
 * A loose file is not a workspace, so it resolves to an empty-window identity.
 */
export async function resolveWorkspaceOpenTarget(
  target: IWorkspaceOpenTarget,
  cwd: string,
  pathService: IWorkspacePathService = nodeWorkspacePathService,
): Promise<IAnyWorkspaceIdentifier> {
  const requestedPath = resolve(cwd, target.path);
  const resolved = await pathService.resolvePath(requestedPath);
  if (
    target.kind === WorkspaceOpenTargetKind.Folder &&
    resolved.kind !== WorkspacePathKind.Directory
  ) {
    throw new Error(`Workspace folder is not a directory: ${target.path}`);
  }
  if (
    target.kind === WorkspaceOpenTargetKind.Workspace &&
    resolved.kind !== WorkspacePathKind.File
  ) {
    throw new Error(`Workspace configuration is not a file: ${target.path}`);
  }

  if (resolved.kind === WorkspacePathKind.Directory) {
    return getSingleFolderWorkspaceIdentifier(URI.file(resolved.path));
  }
  if (
    resolved.kind === WorkspacePathKind.File &&
    (
      target.kind === WorkspaceOpenTargetKind.Workspace ||
      extname(resolved.path).toLowerCase() === ZETA_WORKSPACE_EXTENSION
    )
  ) {
    return getWorkspaceIdentifier(URI.file(resolved.path));
  }
  return UNKNOWN_EMPTY_WINDOW_WORKSPACE;
}

/** Creates the stable identity of one multi-root workspace file. */
export function getWorkspaceIdentifier(
  configPath: URI,
): IWorkspaceIdentifier {
  return Object.freeze({
    id: stableWorkspaceId(configPath),
    configPath,
  });
}

/** Creates the stable identity of one single-folder workspace. */
export function getSingleFolderWorkspaceIdentifier(
  uri: URI,
): ISingleFolderWorkspaceIdentifier {
  return Object.freeze({
    id: stableWorkspaceId(uri),
    uri,
  });
}

function stableWorkspaceId(uri: URI): string {
  const identity = process.platform === "linux"
    ? uri.toString()
    : uri.toString().toLowerCase();
  return createHash("sha256").update(identity).digest("hex");
}
