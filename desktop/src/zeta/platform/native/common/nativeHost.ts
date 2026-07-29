export const NATIVE_HOST_TOGGLE_DEVELOPER_TOOLS_CHANNEL =
  "zeta:native-host:toggle-developer-tools";
export const NATIVE_HOST_OPEN_FOLDER_CHANNEL =
  "zeta:native-host:open-folder";
export const NATIVE_HOST_SET_WINDOW_THEME_CHANNEL =
  "zeta:native-host:set-window-theme";

export interface INativeWindowTheme {
  readonly backgroundColor: string;
  readonly symbolColor: string;
}

/** Window-scoped native capabilities exposed to an Electron renderer. */
export interface INativeHostApi {
  openFolder(): Promise<void>;
  setWindowTheme(theme: INativeWindowTheme): Promise<void>;
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

export function validateNativeWindowTheme(value: unknown): INativeWindowTheme {
  if (typeof value !== "object" || value === null || Array.isArray(value)) throw new Error("window theme must be an object");
  const candidate = value as Record<string, unknown>;
  const keys = Object.keys(candidate).sort();
  if (keys.length !== 2 || keys[0] !== "backgroundColor" || keys[1] !== "symbolColor") throw new Error("window theme contains unknown fields");
  return {
    backgroundColor: validateOpaqueHexColor(candidate.backgroundColor, "backgroundColor"),
    symbolColor: validateOpaqueHexColor(candidate.symbolColor, "symbolColor"),
  };
}

function validateOpaqueHexColor(value: unknown, name: string): string {
  if (typeof value !== "string" || !/^#[0-9a-f]{6}$/i.test(value)) throw new Error(`${name} must be an opaque hexadecimal color`);
  return value.toLowerCase();
}
