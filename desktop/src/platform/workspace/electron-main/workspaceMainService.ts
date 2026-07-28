import { realpath, stat } from "node:fs/promises";
import {
  basename,
  extname,
  resolve,
} from "node:path";
import { URI } from "../../../base/common/uri.js";
import type {
  IpcRoute,
} from "../../app-server/electron-main/trusted-ipc-router.js";
import {
  EMPTY_WORKSPACE,
  type IWorkspaceContext,
  WorkbenchState,
} from "../common/workspace.js";
import {
  WORKSPACE_CONTEXT_READ_CHANNEL,
  validateWorkspaceContextRead,
} from "../common/workspaceIpc.js";

const ZETA_WORKSPACE_EXTENSION = ".zeta-workspace";

export const enum WorkspaceLaunchTargetKind {
  Automatic,
  Folder,
  Workspace,
}

export interface IWorkspaceLaunchTarget {
  readonly kind: WorkspaceLaunchTargetKind;
  readonly path: string;
}

export const enum WorkspacePathKind {
  Directory,
  File,
  Other,
}

export interface IResolvedWorkspacePath {
  readonly kind: WorkspacePathKind;
  readonly path: string;
}

/** Host filesystem operations used to canonicalize a launch target. */
export interface IWorkspacePathService {
  resolvePath(path: string): Promise<IResolvedWorkspacePath>;
}

export interface IResolveStartupWorkspaceOptions {
  readonly arguments: readonly string[];
  readonly cwd: string;
  readonly pathService?: IWorkspacePathService;
}

/**
 * Owns the immutable project identity selected when one native window starts.
 */
export class WorkspaceMainService {
  readonly #workspace: IWorkspaceContext;

  private constructor(workspace: IWorkspaceContext) {
    this.#workspace = workspace;
  }

  static async create(
    options: IResolveStartupWorkspaceOptions,
  ): Promise<WorkspaceMainService> {
    return new WorkspaceMainService(
      await resolveStartupWorkspace(options),
    );
  }

  static empty(): WorkspaceMainService {
    return new WorkspaceMainService(EMPTY_WORKSPACE);
  }

  getWorkspace(): IWorkspaceContext {
    return this.#workspace;
  }
}

/**
 * Parses one project target from user-facing launch arguments.
 *
 * A bare path is classified from the filesystem. Named arguments make the
 * expected target type explicit and reject mismatches.
 */
export function parseWorkspaceLaunchArguments(
  args: readonly string[],
): IWorkspaceLaunchTarget | undefined {
  let target: IWorkspaceLaunchTarget | undefined;
  let positionalOnly = false;

  const accept = (candidate: IWorkspaceLaunchTarget): void => {
    if (target) {
      throw new Error("Zeta can open only one project per window");
    }
    if (candidate.path.trim().length === 0) {
      throw new Error("Workspace path must not be empty");
    }
    target = candidate;
  };

  for (let index = 0; index < args.length; index += 1) {
    const argument = args[index];
    if (!positionalOnly && argument === "--") {
      positionalOnly = true;
      continue;
    }
    if (!positionalOnly && argument === "--folder") {
      const path = args[++index];
      if (path === undefined) {
        throw new Error("--folder requires a path");
      }
      accept({ kind: WorkspaceLaunchTargetKind.Folder, path });
      continue;
    }
    if (!positionalOnly && argument.startsWith("--folder=")) {
      accept({
        kind: WorkspaceLaunchTargetKind.Folder,
        path: argument.slice("--folder=".length),
      });
      continue;
    }
    if (!positionalOnly && argument === "--workspace") {
      const path = args[++index];
      if (path === undefined) {
        throw new Error("--workspace requires a path");
      }
      accept({ kind: WorkspaceLaunchTargetKind.Workspace, path });
      continue;
    }
    if (!positionalOnly && argument.startsWith("--workspace=")) {
      accept({
        kind: WorkspaceLaunchTargetKind.Workspace,
        path: argument.slice("--workspace=".length),
      });
      continue;
    }
    if (!positionalOnly && argument.startsWith("-")) {
      continue;
    }
    accept({ kind: WorkspaceLaunchTargetKind.Automatic, path: argument });
  }

  return target;
}

/** Resolves launch arguments into a canonical empty, folder, or workspace identity. */
export async function resolveStartupWorkspace({
  arguments: args,
  cwd,
  pathService = nodeWorkspacePathService,
}: IResolveStartupWorkspaceOptions): Promise<IWorkspaceContext> {
  const target = parseWorkspaceLaunchArguments(args);
  if (!target) {
    return EMPTY_WORKSPACE;
  }

  const requestedPath = resolve(cwd, target.path);
  const resolved = await pathService.resolvePath(requestedPath);
  if (
    target.kind === WorkspaceLaunchTargetKind.Folder &&
    resolved.kind !== WorkspacePathKind.Directory
  ) {
    throw new Error(`Workspace folder is not a directory: ${target.path}`);
  }
  if (
    target.kind === WorkspaceLaunchTargetKind.Workspace &&
    resolved.kind !== WorkspacePathKind.File
  ) {
    throw new Error(`Workspace configuration is not a file: ${target.path}`);
  }

  if (resolved.kind === WorkspacePathKind.Directory) {
    return Object.freeze({
      state: WorkbenchState.FOLDER,
      uri: URI.file(resolved.path).toString(),
      label: pathLabel(resolved.path),
    });
  }
  if (
    resolved.kind === WorkspacePathKind.File &&
    (
      target.kind === WorkspaceLaunchTargetKind.Workspace ||
      extname(resolved.path).toLowerCase() === ZETA_WORKSPACE_EXTENSION
    )
  ) {
    return Object.freeze({
      state: WorkbenchState.WORKSPACE,
      configUri: URI.file(resolved.path).toString(),
      label: workspaceFileLabel(resolved.path),
    });
  }

  return EMPTY_WORKSPACE;
}

export function workspaceContextIpcRoutes(
  service: WorkspaceMainService,
): readonly IpcRoute<unknown, unknown>[] {
  return [{
    channel: WORKSPACE_CONTEXT_READ_CHANNEL,
    validate: validateWorkspaceContextRead,
    invoke: () => service.getWorkspace(),
  }];
}

const nodeWorkspacePathService: IWorkspacePathService = {
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

function pathLabel(path: string): string {
  return basename(path) || path;
}

function workspaceFileLabel(path: string): string {
  const label = pathLabel(path);
  const hasWorkspaceExtension = label.toLowerCase().endsWith(
    ZETA_WORKSPACE_EXTENSION,
  );
  const withoutExtension = hasWorkspaceExtension
    ? label.slice(0, -ZETA_WORKSPACE_EXTENSION.length)
    : label;
  return withoutExtension || label;
}
