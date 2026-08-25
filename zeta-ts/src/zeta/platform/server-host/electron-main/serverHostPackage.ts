import { join } from "node:path";
import { developmentArtifactsPath } from "../../environment/node/developmentArtifacts.js";

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
	return join(serverHostPackageRoot(location), "bin", location.platform === "win32" ? "zeta-server.exe" : "zeta-server");
}

/** Resolves the profile-scoped App Server daemon from the canonical Desktop package layout. */
export function appServerDaemonExecutablePath(location: ServerHostPackageLocation): string {
	return join(serverHostPackageRoot(location), "bin", location.platform === "win32" ? "zeta-app-server-daemon.exe" : "zeta-app-server-daemon");
}

function serverHostPackageRoot(location: ServerHostPackageLocation): string {
	const packageRoot = location.isPackaged
		? location.resourcesPath
		: developmentArtifactsPath(location.appPath, "dev", "zeta-package");
	return packageRoot;
}

/** Resolves the development-only generation pointer published by the Rust watcher. */
export function developmentServerHostGenerationPath(appPath: string): string {
	return developmentArtifactsPath(appPath, "dev", "server-host", "current.json");
}
