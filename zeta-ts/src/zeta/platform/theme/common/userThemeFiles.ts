export const USER_THEME_FILES_LIST_CHANNEL = "zeta:user-themes:list";
export const USER_THEME_FILE_WRITE_CHANNEL = "zeta:user-themes:write";
export const USER_THEME_FILE_DELETE_CHANNEL = "zeta:user-themes:delete";

export type UserThemeFileWriteOperation = "create" | "replace";

export interface IUserThemeFile {
	readonly name: string;
	readonly content?: string;
	readonly error?: string;
}

export interface IUserThemeFileList {
	readonly directory: string;
	readonly files: readonly IUserThemeFile[];
}

export interface IUserThemeFilesApi {
	delete(request: IUserThemeFileDeleteRequest): Promise<unknown>;
	list(): Promise<unknown>;
	write(request: IUserThemeFileWriteRequest): Promise<unknown>;
}

export interface IUserThemeFileDeleteRequest {
	readonly file: string;
	readonly themeId: string;
}

export interface IUserThemeFileWriteRequest {
	readonly content: string;
	readonly file: string;
	readonly operation: UserThemeFileWriteOperation;
}

export function validateUserThemeFileList(value: unknown): IUserThemeFileList {
	const candidate = exactRecord(value, ["directory", "files"]);
	if (typeof candidate.directory !== "string" || candidate.directory.length < 1 || candidate.directory.length > 4096) throw new Error("User theme directory must be a non-empty path");
	if (!Array.isArray(candidate.files) || candidate.files.length > 128) throw new Error("User theme file list must contain at most 128 entries");
	return Object.freeze({
		directory: candidate.directory,
		files: Object.freeze(candidate.files.map(validateUserThemeFile)),
	});
}

export function validateUserThemeFileWriteRequest(value: unknown): IUserThemeFileWriteRequest {
	const candidate = exactRecord(value, ["content", "file", "operation"]);
	if (typeof candidate.file !== "string" || !isUserThemeFileName(candidate.file)) throw new Error("User theme filename is invalid");
	if (typeof candidate.content !== "string" || candidate.content.length < 1 || candidate.content.length > 1_048_576) throw new Error("User theme content must contain between 1 byte and 1 MiB");
	if (candidate.operation !== "create" && candidate.operation !== "replace") throw new Error("User theme write operation must be 'create' or 'replace'");
	return Object.freeze({
		content: candidate.content,
		file: candidate.file,
		operation: candidate.operation,
	});
}

export function validateUserThemeFileDeleteRequest(value: unknown): IUserThemeFileDeleteRequest {
	const candidate = exactRecord(value, ["file", "themeId"]);
	if (typeof candidate.file !== "string" || !isUserThemeFileName(candidate.file)) throw new Error("User theme filename is invalid");
	if (typeof candidate.themeId !== "string" || !/^[a-z0-9]+(?:-[a-z0-9]+)*$/.test(candidate.themeId)) throw new Error("User theme id is invalid");
	return Object.freeze({
		file: candidate.file,
		themeId: candidate.themeId,
	});
}

export function isUserThemeFileName(value: string): boolean {
	return /^[^\\/:*?"<>|]{1,250}\.json$/i.test(value);
}

function validateUserThemeFile(value: unknown): IUserThemeFile {
	const candidate = exactRecord(value, ["content", "error", "name"]);
	if (typeof candidate.name !== "string" || !isUserThemeFileName(candidate.name)) throw new Error("User theme filename is invalid");
	const hasContent = typeof candidate.content === "string";
	const hasError = typeof candidate.error === "string";
	if (hasContent === hasError) throw new Error(`User theme '${candidate.name}' must contain exactly one of content or error`);
	if (hasContent && (candidate.content as string).length > 1_048_576) throw new Error(`User theme '${candidate.name}' exceeds the 1 MiB limit`);
	if (hasError && ((candidate.error as string).length < 1 || (candidate.error as string).length > 512)) throw new Error(`User theme '${candidate.name}' has an invalid error`);
	return Object.freeze({
		name: candidate.name,
		...(hasContent ? { content: candidate.content as string } : { error: candidate.error as string }),
	});
}

function exactRecord(value: unknown, allowedKeys: readonly string[]): Record<string, unknown> {
	if (typeof value !== "object" || value === null || Array.isArray(value)) throw new Error("Value must be an object");
	const candidate = value as Record<string, unknown>;
	const allowed = new Set(allowedKeys);
	if (Object.keys(candidate).some((key) => !allowed.has(key))) throw new Error("Value contains unknown fields");
	return candidate;
}
