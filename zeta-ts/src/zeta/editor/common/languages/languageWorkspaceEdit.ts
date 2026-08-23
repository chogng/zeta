import { type URI } from "../../../base/common/uri.js";
import { type TextEdit } from "../core/text.js";

/** Text replacements for one exact resource snapshot. */
export interface LanguageTextDocumentEdit {
	readonly kind: "textDocument";
	readonly resource: URI;
	/** Optional model version used to reject stale edits for an open document. */
	readonly version?: number;
	/** Optional exact content baseline used to reject stale closed or cross-process resources. */
	readonly expectedText?: string;
	readonly edits: readonly TextEdit[];
}

export type LanguageExistingTargetBehavior = "error" | "overwrite" | "ignore";
export type LanguageMissingTargetBehavior = "error" | "ignore";
export type LanguageDeleteMode = "fileOrEmptyDirectory" | "recursive";

export interface LanguageCreateFileEdit {
	readonly kind: "create";
	readonly resource: URI;
	readonly existing: LanguageExistingTargetBehavior;
}

export interface LanguageRenameFileEdit {
	readonly kind: "rename";
	readonly source: URI;
	readonly target: URI;
	readonly existing: LanguageExistingTargetBehavior;
}

export interface LanguageDeleteFileEdit {
	readonly kind: "delete";
	readonly resource: URI;
	readonly missing: LanguageMissingTargetBehavior;
	readonly mode: LanguageDeleteMode;
}

export type LanguageWorkspaceEditEntry = LanguageTextDocumentEdit | LanguageCreateFileEdit | LanguageRenameFileEdit | LanguageDeleteFileEdit;

/** One language-server edit spanning one or more text resources. */
export interface LanguageWorkspaceEdit {
	readonly entries: readonly LanguageWorkspaceEditEntry[];
}

export function normalizeLanguageWorkspaceEdit(edit: LanguageWorkspaceEdit): LanguageWorkspaceEdit {
	if (!edit || typeof edit !== "object" || !Array.isArray(edit.entries)) throw new TypeError("Language workspace edit must contain ordered entries");
	const entries = edit.entries.map(entry => {
		if (!entry || typeof entry !== "object") throw new TypeError("Language workspace edit entry must be an object");
		switch (entry.kind) {
			case "textDocument":
				if (!entry.resource || !Array.isArray(entry.edits)) throw new TypeError("Language document edit requires a resource and text edits");
				if (entry.version !== undefined && (!Number.isSafeInteger(entry.version) || entry.version < 1)) throw new RangeError("Language document edit version must be a positive safe integer");
				if (entry.expectedText !== undefined && typeof entry.expectedText !== "string") throw new TypeError("Language document edit expected text must be text");
				return Object.freeze({ kind: entry.kind, resource: entry.resource, ...(entry.version !== undefined ? { version: entry.version } : {}), ...(entry.expectedText !== undefined ? { expectedText: entry.expectedText } : {}), edits: Object.freeze([...entry.edits]) });
			case "create":
				return Object.freeze({ kind: entry.kind, resource: requireResource(entry.resource, "create target"), existing: existingBehavior(entry.existing) });
			case "rename":
				return Object.freeze({ kind: entry.kind, source: requireResource(entry.source, "rename source"), target: requireResource(entry.target, "rename target"), existing: existingBehavior(entry.existing) });
			case "delete":
				if (entry.missing !== "error" && entry.missing !== "ignore") throw new TypeError("Language delete missing-target behavior is invalid");
				if (entry.mode !== "fileOrEmptyDirectory" && entry.mode !== "recursive") throw new TypeError("Language delete mode is invalid");
				return Object.freeze({ kind: entry.kind, resource: requireResource(entry.resource, "delete target"), missing: entry.missing, mode: entry.mode });
			default:
				throw new TypeError("Language workspace edit entry kind is invalid");
		}
	});
	return Object.freeze({ entries: Object.freeze(entries) });
}

function requireResource(resource: URI, name: string): URI {
	if (!resource || typeof resource.toString !== "function") throw new TypeError(`Language workspace ${name} requires a resource`);
	return resource;
}

function existingBehavior(value: LanguageExistingTargetBehavior): LanguageExistingTargetBehavior {
	if (value !== "error" && value !== "overwrite" && value !== "ignore") throw new TypeError("Language workspace existing-target behavior is invalid");
	return value;
}
