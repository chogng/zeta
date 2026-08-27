import { constants } from "node:fs";
import { access, copyFile, cp, mkdir } from "node:fs/promises";
import { homedir } from "node:os";
import { join, posix, win32 } from "node:path";

export interface LocalProfileRootOptions {
	readonly environment: Readonly<Record<string, string | undefined>>;
	readonly homeDirectory: string;
	readonly platform: NodeJS.Platform;
}

export interface LegacyLocalProfileMigrationOptions {
	readonly legacyUserDataRoot: string;
	readonly profileRoot: string;
}

/** Resolves the cross-product user profile shared by Desktop, Zeta Code, and app. */
export function resolveLocalProfileRoot(options: LocalProfileRootOptions): string {
	const path = options.platform === "win32" ? win32 : posix;
	const configured = options.environment.ZETA_PROFILE_ROOT;
	if (configured !== undefined) {
		if (!configured || !path.isAbsolute(configured)) throw new Error("ZETA_PROFILE_ROOT must be an absolute path");
		return path.normalize(configured);
	}
	if (!options.homeDirectory || !path.isAbsolute(options.homeDirectory)) throw new Error("The user home directory must be an absolute path");
	return path.join(options.homeDirectory, ".zeta");
}

export function localProfileRoot(): string {
	return resolveLocalProfileRoot({ environment: process.env, homeDirectory: homedir(), platform: process.platform });
}

/** Copies legacy Desktop resources only when their canonical destination does not exist. */
export async function migrateLegacyLocalProfile(options: LegacyLocalProfileMigrationOptions): Promise<void> {
	await mkdir(options.profileRoot, { recursive: true });
	await copyFileIfMissing(join(options.legacyUserDataRoot, "configuration.json"), join(options.profileRoot, "configuration.json"));
	await copyFileIfMissing(join(options.legacyUserDataRoot, "keybindings.json"), join(options.profileRoot, "keybindings.json"));
	await copyFileIfMissing(join(options.legacyUserDataRoot, "keyboard-layout.json"), join(options.profileRoot, "keyboard-layout.json"));
	await copyDirectoryIfMissing(join(options.legacyUserDataRoot, "themes"), join(options.profileRoot, "themes"));
}

async function copyFileIfMissing(source: string, destination: string): Promise<void> {
	try {
		await copyFile(source, destination, constants.COPYFILE_EXCL);
	} catch (error) {
		if (!isMissingOrExistingPathError(error)) throw error;
	}
}

async function copyDirectoryIfMissing(source: string, destination: string): Promise<void> {
	if (await pathExists(destination) || !await pathExists(source)) return;
	await cp(source, destination, { recursive: true, force: false, errorOnExist: true });
}

async function pathExists(path: string): Promise<boolean> {
	try {
		await access(path);
		return true;
	} catch (error) {
		if (isNodeError(error) && error.code === "ENOENT") return false;
		throw error;
	}
}

function isMissingOrExistingPathError(error: unknown): boolean {
	return isNodeError(error) && (error.code === "ENOENT" || error.code === "EEXIST");
}

function isNodeError(error: unknown): error is NodeJS.ErrnoException {
	return error instanceof Error && "code" in error;
}
