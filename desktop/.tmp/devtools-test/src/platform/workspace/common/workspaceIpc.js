export const WORKSPACE_CONTEXT_READ_CHANNEL = "zeta:workspace:context:read";
export function validateWorkspaceContextRead(value) {
    if (value !== undefined) {
        throw new Error("workspace context read does not accept parameters");
    }
    return undefined;
}
