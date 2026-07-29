export const NATIVE_HOST_TOGGLE_DEVELOPER_TOOLS_CHANNEL =
  "zeta:native-host:toggle-developer-tools";
export const NATIVE_HOST_OPEN_FOLDER_CHANNEL =
  "zeta:native-host:open-folder";

/** Window-scoped native capabilities exposed to an Electron renderer. */
export interface INativeHostApi {
  openFolder(): Promise<void>;
  toggleDeveloperTools(): Promise<void>;
}

export function validateOpenFolder(value: unknown): undefined {
  if (value !== undefined) {
    throw new Error("open folder does not accept parameters");
  }
  return undefined;
}

export function validateToggleDeveloperTools(value: unknown): undefined {
  if (value !== undefined) {
    throw new Error("toggle developer tools does not accept parameters");
  }
  return undefined;
}
