import { lstatSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { APP_SERVER_PROTOCOL_MAJOR, APP_SERVER_PROTOCOL_REVISION, APP_SERVER_SCHEMA_HASH } from "../../../../../generated/app-server/types.js";
import { developmentArtifactsPath } from "../../environment/node/developmentArtifacts.js";

export interface ServerHostPackageLocation {
	readonly appPath: string;
	readonly expectedVersion?: string;
	readonly isPackaged: boolean;
	readonly platform: NodeJS.Platform;
	readonly resourcesPath: string;
}

interface ZetaPackageMetadata {
	readonly buildId?: unknown;
	readonly components?: {
		readonly serverHost?: { readonly binarySha256?: unknown };
	};
	readonly entrypoint?: unknown;
	readonly layoutVersion?: unknown;
	readonly protocol?: {
		readonly major?: unknown;
		readonly revision?: unknown;
		readonly schemaHash?: unknown;
	};
	readonly version?: unknown;
}

/**
 * Resolves the product-neutral backend host from the canonical Desktop package layout.
 */
export function serverHostExecutablePath(location: ServerHostPackageLocation): string {
	return join(serverHostPackageRoot(location), "bin", location.platform === "win32" ? "zeta-server.exe" : "zeta-server");
}

/** Reads the digest bound to the signed product package; development generations use protocol negotiation. */
export function packagedServerHostSha256(location: ServerHostPackageLocation): string | undefined {
	if (!location.isPackaged) return undefined;
	const packageRoot = serverHostPackageRoot(location);
	const metadataPath = join(packageRoot, "zeta-package.json");
	const metadataStat = lstatSync(metadataPath);
	if (!metadataStat.isFile() || metadataStat.isSymbolicLink() || metadataStat.size > 1024 * 1024) {
		throw new Error(`Invalid Zeta package metadata file: ${metadataPath}`);
	}
	const metadata = JSON.parse(readFileSync(metadataPath, "utf8")) as ZetaPackageMetadata;
	const expectedEntrypoint = `bin/${location.platform === "win32" ? "zeta-server.exe" : "zeta-server"}`;
	const digest = metadata.components?.serverHost?.binarySha256;
	const protocolMatchesDesktop = metadata.protocol?.major === APP_SERVER_PROTOCOL_MAJOR
		&& metadata.protocol.revision === APP_SERVER_PROTOCOL_REVISION
		&& metadata.protocol.schemaHash === APP_SERVER_SCHEMA_HASH;
	if (metadata.layoutVersion !== 2 || metadata.entrypoint !== expectedEntrypoint || (location.expectedVersion !== undefined && metadata.version !== location.expectedVersion) || !protocolMatchesDesktop || typeof metadata.buildId !== "string" || !/^sha256:[a-f0-9]{64}$/.test(metadata.buildId) || typeof digest !== "string" || !/^[a-f0-9]{64}$/.test(digest)) {
		throw new Error(`Invalid Zeta package metadata: ${metadataPath}`);
	}
	return digest;
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
