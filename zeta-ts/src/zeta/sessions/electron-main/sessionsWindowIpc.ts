import type { IpcRoute } from "../../platform/ipc/electron-main/trustedIpcRouter.js";
import { OPEN_SESSION_WORKSPACE_CHANNEL, OPEN_SESSIONS_WINDOW_CHANNEL, RETURN_TO_WORKBENCH_CHANNEL, validateSessionWorkspaceRoot, validateSessionsWindowCommand } from "../common/sessionsWindow.js";

/** Main-process operations that own the dedicated Sessions window lifecycle. */
export interface SessionsWindowIpcHandlers {
	openSessionsWindow(): void | Promise<void>;
	returnToWorkbench(): void | Promise<void>;
	openWorkspace(root: string): void | Promise<void>;
}

/** Binds a renderer to the Sessions window operations scoped to its BrowserWindow. */
export function sessionsWindowIpcRoutes(handlers: SessionsWindowIpcHandlers): readonly IpcRoute<unknown, unknown>[] {
	return [
		{
			channel: OPEN_SESSIONS_WINDOW_CHANNEL,
			validate: validateSessionsWindowCommand,
			invoke: () => handlers.openSessionsWindow(),
		},
		{
			channel: RETURN_TO_WORKBENCH_CHANNEL,
			validate: validateSessionsWindowCommand,
			invoke: () => handlers.returnToWorkbench(),
		},
		{
			channel: OPEN_SESSION_WORKSPACE_CHANNEL,
			validate: validateSessionWorkspaceRoot,
			invoke: (root) => handlers.openWorkspace(root as string),
		},
	];
}
