import { join, resolve } from "node:path";

export interface AppServerPackageLocation {
  readonly appPath: string;
  readonly isPackaged: boolean;
  readonly platform: NodeJS.Platform;
  readonly resourcesPath: string;
}

/**
 * Resolves the App Server executable from the same canonical package layout in
 * both Desktop development and production hosts.
 */
export function appServerExecutablePath(location: AppServerPackageLocation): string {
  const packageRoot = location.isPackaged
    ? location.resourcesPath
    : resolve(location.appPath, ".tmp", "zeta-package");
  const executableName = location.platform === "win32" ? "zeta.exe" : "zeta";
  return join(packageRoot, "bin", executableName);
}
