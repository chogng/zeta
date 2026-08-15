import { join, resolve } from "node:path";

export interface ServerHostPackageLocation {
  readonly appPath: string;
  readonly isPackaged: boolean;
  readonly platform: NodeJS.Platform;
  readonly resourcesPath: string;
}

/**
 * Resolves the product-neutral backend host from the canonical Desktop package layout.
 */
export function serverHostExecutablePath(location: ServerHostPackageLocation): string {
  const packageRoot = location.isPackaged
    ? location.resourcesPath
    : resolve(location.appPath, ".tmp", "zeta-package");
  const executableName = location.platform === "win32" ? "zeta-server.exe" : "zeta-server";
  return join(packageRoot, "bin", executableName);
}
