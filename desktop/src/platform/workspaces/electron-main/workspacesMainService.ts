import {
  type IAnyWorkspaceIdentifier,
  UNKNOWN_EMPTY_WINDOW_WORKSPACE,
  serializeWorkspaceIdentifier,
} from "../../workspace/common/workspace.js";
import {
  WORKSPACE_CONTEXT_READ_CHANNEL,
  validateWorkspaceContextRead,
} from "../../workspace/common/workspaceIpc.js";
import type {
  IpcRoute,
} from "../../app-server/electron-main/trusted-ipc-router.js";
import {
  type IWorkspaceOpenTarget,
  WorkspaceOpenTargetKind,
} from "../common/workspaces.js";
import {
  type IWorkspacePathService,
  nodeWorkspacePathService,
  resolveWorkspaceOpenTarget,
} from "../node/workspaces.js";

export interface IResolveStartupWorkspaceOptions {
  readonly arguments: readonly string[];
  readonly cwd: string;
}

/**
 * Resolves user-facing workspace targets for native windows.
 *
 * The opened workspace remains window-owned; this service does not duplicate
 * current-window context state.
 */
export class WorkspacesMainService {
  readonly #pathService: IWorkspacePathService;

  constructor(
    pathService: IWorkspacePathService = nodeWorkspacePathService,
  ) {
    this.#pathService = pathService;
  }

  async resolveStartupWorkspace({
    arguments: args,
    cwd,
  }: IResolveStartupWorkspaceOptions): Promise<IAnyWorkspaceIdentifier> {
    const target = parseWorkspaceLaunchArguments(args);
    if (!target) {
      return UNKNOWN_EMPTY_WINDOW_WORKSPACE;
    }
    return resolveWorkspaceOpenTarget(target, cwd, this.#pathService);
  }
}

/** Exposes one window-owned workspace identity through the trusted IPC router. */
export function workspaceContextIpcRoutes(
  workspace: IAnyWorkspaceIdentifier,
): readonly IpcRoute<unknown, unknown>[] {
  return [{
    channel: WORKSPACE_CONTEXT_READ_CHANNEL,
    validate: validateWorkspaceContextRead,
    invoke: () => serializeWorkspaceIdentifier(workspace),
  }];
}

/**
 * Parses one project target from user-facing launch arguments.
 *
 * A bare path is classified from the filesystem. Named arguments make the
 * expected target type explicit and reject mismatches.
 */
export function parseWorkspaceLaunchArguments(
  args: readonly string[],
): IWorkspaceOpenTarget | undefined {
  let target: IWorkspaceOpenTarget | undefined;
  let positionalOnly = false;

  const accept = (candidate: IWorkspaceOpenTarget): void => {
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
      accept({ kind: WorkspaceOpenTargetKind.Folder, path });
      continue;
    }
    if (!positionalOnly && argument.startsWith("--folder=")) {
      accept({
        kind: WorkspaceOpenTargetKind.Folder,
        path: argument.slice("--folder=".length),
      });
      continue;
    }
    if (!positionalOnly && argument === "--workspace") {
      const path = args[++index];
      if (path === undefined) {
        throw new Error("--workspace requires a path");
      }
      accept({ kind: WorkspaceOpenTargetKind.Workspace, path });
      continue;
    }
    if (!positionalOnly && argument.startsWith("--workspace=")) {
      accept({
        kind: WorkspaceOpenTargetKind.Workspace,
        path: argument.slice("--workspace=".length),
      });
      continue;
    }
    if (!positionalOnly && argument.startsWith("-")) {
      continue;
    }
    accept({ kind: WorkspaceOpenTargetKind.Automatic, path: argument });
  }

  return target;
}
