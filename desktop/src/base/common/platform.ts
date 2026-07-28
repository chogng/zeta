/** Runtime platforms supported by the workbench. Web is intentionally distinct from native OS targets. */
export enum Platform {
  Web = "web",
  Windows = "windows",
  Mac = "mac",
  Linux = "linux",
}

/** Host operating systems that affect keybinding resolution and labels. */
export enum OperatingSystem {
  Windows = "windows",
  Macintosh = "mac",
  Linux = "linux",
}

interface ProcessLike {
  platform?: string;
  versions?: { electron?: string };
}

const processLike = (globalThis as typeof globalThis & { process?: ProcessLike }).process;
const userAgent = globalThis.navigator?.userAgent ?? "";
const nativeRuntime = Boolean(processLike?.versions?.electron) || /Electron\//.test(userAgent) || typeof globalThis.navigator === "undefined";
const systemPlatform = processLike?.platform ?? globalThis.navigator?.platform ?? userAgent;
const windowsSystem = /win/i.test(systemPlatform);
const macintoshSystem = /mac/i.test(systemPlatform);

export const isNative = nativeRuntime;
export const isWeb = !isNative;
export const isWindows = isNative && windowsSystem;
export const isMacintosh = isNative && macintoshSystem;
export const isLinux = isNative && /linux/i.test(systemPlatform);

/** The platform detected once for the current runtime. */
export const platform = isWeb
  ? Platform.Web
  : isWindows
    ? Platform.Windows
    : isMacintosh
      ? Platform.Mac
      : Platform.Linux;

/**
 * The host OS detected independently from the runtime platform.
 *
 * A browser running on macOS is still `Platform.Web`, but keyboard shortcuts
 * must use Command and macOS labels.
 */
export const operatingSystem = windowsSystem
  ? OperatingSystem.Windows
  : macintoshSystem
    ? OperatingSystem.Macintosh
    : OperatingSystem.Linux;
