import type {
  IpcRoute,
} from "../../app-server/electron-main/trusted-ipc-router.js";
import {
  type IAnyWorkspaceIdentifier,
  serializeWorkspaceIdentifier,
} from "../common/workspace.js";
import {
  WORKSPACE_CONTEXT_READ_CHANNEL,
  validateWorkspaceContextRead,
} from "../common/workspaceIpc.js";

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
