export const NATIVE_HOST_TOGGLE_DEVELOPER_TOOLS_CHANNEL =
  "zeta:native-host:toggle-developer-tools";

/** Window-scoped native capabilities exposed to an Electron renderer. */
export interface INativeHostApi {
  toggleDeveloperTools(): Promise<void>;
}

export function validateToggleDeveloperTools(value: unknown): undefined {
  if (value !== undefined) {
    throw new Error("toggle developer tools does not accept parameters");
  }
  return undefined;
}
