import { APP_SERVER_METHODS, type GitCommitParams, type GitPathsParams } from "../../../../../generated/app-server/types.js";
import type { AppServerSupervisor } from "../../app-server/electron-main/app-server-supervisor.js";
import { record, string } from "../../ipc/electron-main/ipcValidation.js";
import type { IpcRoute } from "../../ipc/electron-main/trustedIpcRouter.js";
import { relativeWorkspacePath } from "../../workspace/electron-main/workspacePathValidation.js";

/** Exact-shape IPC routes for Git query and mutation operations. */
export function gitIpcRoutes(supervisor: AppServerSupervisor): readonly IpcRoute<unknown, unknown>[] {
  return [
    route({
      channel: "zeta:git:status",
      validate: emptyParams,
      invoke: () => supervisor.request(APP_SERVER_METHODS["git/status"], {}),
    }),
    route({
      channel: "zeta:git:history",
      validate: emptyParams,
      invoke: () => supervisor.request(APP_SERVER_METHODS["git/history"], {}),
    }),
    route({
      channel: "zeta:git:stage",
      validate: gitPathsParams,
      invoke: (params) => supervisor.request(APP_SERVER_METHODS["git/stage"], params),
    }),
    route({
      channel: "zeta:git:unstage",
      validate: gitPathsParams,
      invoke: (params) => supervisor.request(APP_SERVER_METHODS["git/unstage"], params),
    }),
    route({
      channel: "zeta:git:discard-worktree",
      validate: gitPathsParams,
      invoke: (params) => supervisor.request(APP_SERVER_METHODS["git/discardWorktree"], params),
    }),
    route({
      channel: "zeta:git:commit",
      validate: gitCommitParams,
      invoke: (params) => supervisor.request(APP_SERVER_METHODS["git/commit"], params),
    }),
    route({
      channel: "zeta:git:fetch",
      validate: emptyParams,
      invoke: () => supervisor.request(APP_SERVER_METHODS["git/fetch"], {}),
    }),
    route({
      channel: "zeta:git:pull",
      validate: emptyParams,
      invoke: () => supervisor.request(APP_SERVER_METHODS["git/pull"], {}),
    }),
    route({
      channel: "zeta:git:push",
      validate: emptyParams,
      invoke: () => supervisor.request(APP_SERVER_METHODS["git/push"], {}),
    }),
  ];
}

function route<P, R>(definition: IpcRoute<P, R>): IpcRoute<unknown, unknown> {
  return {
    channel: definition.channel,
    validate: definition.validate,
    invoke: (params) => definition.invoke(params as P),
  };
}

function emptyParams(value: unknown): Record<string, never> {
  if (value === undefined) return {};
  return record(value, []) as Record<string, never>;
}

function gitPathsParams(value: unknown): GitPathsParams {
  const params = record(value, ["paths"]);
  if (!Array.isArray(params.paths) || params.paths.length === 0 || params.paths.length > 5_000) {
    throw new Error("paths must contain between 1 and 5000 entries");
  }
  return {
    paths: params.paths.map((path, index) => {
      const resolved = relativeWorkspacePath(path);
      if (!resolved) throw new Error(`paths[${index}] must not be empty`);
      return resolved;
    }),
  };
}

function gitCommitParams(value: unknown): GitCommitParams {
  const params = record(value, ["message"]);
  const message = string(params.message, "message");
  if (!message.trim() || message.includes("\0") || new TextEncoder().encode(message).byteLength > 65_536) {
    throw new Error("message must be non-empty, NUL-free, and no larger than 65536 UTF-8 bytes");
  }
  return { message };
}
