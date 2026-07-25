/** Runtime platforms supported by the workbench. Web is intentionally distinct from native OS targets. */
export enum Platform {
  Web = "web",
  Windows = "windows",
  Mac = "mac",
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

export const isNative = nativeRuntime;
export const isWeb = !isNative;
export const isWindows = isNative && /win/i.test(systemPlatform);
export const isMacintosh = isNative && /mac/i.test(systemPlatform);
export const isLinux = isNative && /linux/i.test(systemPlatform);

/** The platform detected once for the current runtime. */
export const platform = isWeb
  ? Platform.Web
  : isWindows
    ? Platform.Windows
    : isMacintosh
      ? Platform.Mac
      : Platform.Linux;
