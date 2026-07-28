export const WORKSPACE_CONTEXT_READ_CHANNEL =
  "zeta:workspace:context:read";

/** Narrow host capability that exposes one serialized window workspace. */
export interface IWorkspaceContextApi {
  getWorkspace(): Promise<unknown>;
}

export function validateWorkspaceContextRead(value: unknown): undefined {
  if (value !== undefined) {
    throw new Error("workspace context read does not accept parameters");
  }
  return undefined;
}
