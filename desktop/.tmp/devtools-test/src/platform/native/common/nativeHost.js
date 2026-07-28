export const NATIVE_HOST_TOGGLE_DEVELOPER_TOOLS_CHANNEL = "zeta:native-host:toggle-developer-tools";
export function validateToggleDeveloperTools(value) {
    if (value !== undefined) {
        throw new Error("toggle developer tools does not accept parameters");
    }
    return undefined;
}
