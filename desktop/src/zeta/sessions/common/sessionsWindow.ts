/** Renderer capability for opening and returning from the dedicated Sessions window. */
export interface ISessionsWindowApi {
  openSessionsWindow(): Promise<void>;
  returnToWorkbench(): Promise<void>;
  openWorkspace(root: string): Promise<void>;
}

export const OPEN_SESSIONS_WINDOW_CHANNEL = "zeta:sessions-window:open";
export const RETURN_TO_WORKBENCH_CHANNEL = "zeta:sessions-window:return-to-workbench";
export const OPEN_SESSION_WORKSPACE_CHANNEL = "zeta:sessions-window:open-workspace";

export function validateSessionWorkspaceRoot(value: unknown): string {
  if (typeof value !== "string" || value.trim().length === 0) {
    throw new TypeError("Session Workspace root must be a non-empty string");
  }
  return value;
}

/** Validates a window command that deliberately carries no renderer-controlled data. */
export function validateSessionsWindowCommand(value: unknown): undefined {
  if (value !== undefined) {
    throw new TypeError("Sessions window commands do not accept parameters");
  }
  return undefined;
}
