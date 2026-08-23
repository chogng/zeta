import { lstat, mkdir, readFile, readdir, rename, rm, writeFile } from "node:fs/promises";
import { join } from "node:path";
import { parseUserColorTheme } from "../common/userColorTheme.js";
import { isUserThemeFileName, type IUserThemeFileDeleteRequest, type IUserThemeFileList, type IUserThemeFileWriteRequest } from "../common/userThemeFiles.js";

const MAX_THEME_FILES = 128;
const MAX_THEME_FILE_BYTES = 1_048_576;

/** Discovers bounded regular JSON files inside one host-owned theme directory. */
export class UserThemeFileService {
	constructor(readonly directory: string) {
		if (!directory) throw new TypeError("User theme directory must not be empty");
	}

	async list(): Promise<IUserThemeFileList> {
		await mkdir(this.directory, { recursive: true });
		const entries = (await readdir(this.directory, { withFileTypes: true }))
			.filter((entry) => entry.isFile() && entry.name.toLowerCase().endsWith(".json"))
			.sort((left, right) => left.name.localeCompare(right.name))
			.slice(0, MAX_THEME_FILES);
		const files = await Promise.all(entries.map(async ({ name }) => {
			const path = join(this.directory, name);
			try {
				const metadata = await lstat(path);
				if (!metadata.isFile()) return { name, error: "Theme path is not a regular file" };
				if (metadata.size > MAX_THEME_FILE_BYTES) return { name, error: "Theme file exceeds the 1 MiB limit" };
				return { name, content: await readFile(path, "utf8") };
			} catch {
				return { name, error: "Unable to read theme file" };
			}
		}));
		return { directory: this.directory, files };
	}

	async write(request: IUserThemeFileWriteRequest): Promise<IUserThemeFileList> {
		if (!isUserThemeFileName(request.file)) throw new Error("User theme filename is invalid");
		const theme = parseUserColorTheme(request.content);
		if (request.operation === "create" && request.file !== `${theme.id}.json`) {
			throw new Error("A new user theme filename must match its theme id");
		}
		await mkdir(this.directory, { recursive: true });
		const path = join(this.directory, request.file);
		if (request.operation === "create") {
			await writeFile(path, request.content, { encoding: "utf8", flag: "wx" });
		} else {
			const metadata = await lstat(path);
			if (!metadata.isFile()) throw new Error("User theme path is not a regular file");
			const existingTheme = parseUserColorTheme(await readFile(path, "utf8"));
			if (existingTheme.id !== theme.id) throw new Error("Replacing a user theme cannot change its id");
			const temporaryPath = `${path}.${process.pid}.tmp`;
			try {
				await writeFile(temporaryPath, request.content, "utf8");
				await rename(temporaryPath, path);
			} finally {
				await rm(temporaryPath, { force: true });
			}
		}
		return this.list();
	}

	async delete(request: IUserThemeFileDeleteRequest): Promise<IUserThemeFileList> {
		if (!isUserThemeFileName(request.file)) throw new Error("User theme filename is invalid");
		const path = join(this.directory, request.file);
		const metadata = await lstat(path);
		if (!metadata.isFile()) throw new Error("User theme path is not a regular file");
		const theme = parseUserColorTheme(await readFile(path, "utf8"));
		if (theme.id !== request.themeId) throw new Error("User theme file does not match the requested theme id");
		await rm(path);
		return this.list();
	}
}
