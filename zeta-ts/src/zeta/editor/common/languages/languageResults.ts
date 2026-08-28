import { TextRange, type TextPosition, type TextSnapshot } from "../core/text.js";
import { VersionedLanguageResultStore } from "./languageResultStore.js";
import { type TextModel } from "../model/textModel.js";

export { attachLanguageTokenResultDelta, getLanguageTokenResultDelta } from '../services/semanticTokensDto.js';
export type { LanguageTokenResultDelta, LanguageTokenResultSplice } from '../services/semanticTokensDto.js';
export { createLanguageTokenSnapshotNormalizer, createLanguageTokenStore } from "../tokens/languageTokens.js";
export type { LanguageToken, LanguageTokenResult } from "../tokens/languageTokens.js";

export enum LanguageDiagnosticSeverity {
	Error = "error",
	Warning = "warning",
	Information = "information",
	Hint = "hint",
}

export type LanguageDiagnosticCode = string | number;

export interface LanguageDiagnostic {
	readonly range: TextRange;
	readonly severity: LanguageDiagnosticSeverity;
	readonly message: string;
	readonly code?: LanguageDiagnosticCode;
	readonly source?: string;
}

export interface LanguageDiagnosticResult {
	readonly diagnostics: readonly LanguageDiagnostic[];
}

export function createLanguageDiagnosticStore(model: TextModel): VersionedLanguageResultStore<LanguageDiagnosticResult> {
	return new VersionedLanguageResultStore(model, (value, currentModel) => normalizeLanguageDiagnosticResult(
		value,
		range => assertModelRange(currentModel, range, "Language diagnostic"),
	));
}

export function createLanguageDiagnosticSnapshotNormalizer(snapshot: TextSnapshot): (value: LanguageDiagnosticResult) => LanguageDiagnosticResult {
	const lines = snapshot.getText().split("\n");
	return value => normalizeLanguageDiagnosticResult(value, range => assertSnapshotRange(lines, range, "Language diagnostic"));
}

function normalizeLanguageDiagnosticResult(value: LanguageDiagnosticResult, validateRange: (range: TextRange) => void): LanguageDiagnosticResult {
	if (typeof value !== "object" || value === null || !Array.isArray(value.diagnostics)) {
		throw new TypeError("Language diagnostic result must contain a diagnostics array");
	}
	const diagnostics = value.diagnostics.map(diagnostic => {
		if (typeof diagnostic !== "object" || diagnostic === null) {
			throw new TypeError("Language diagnostic must be an object");
		}
		validateRange(diagnostic.range);
		if (!Object.values(LanguageDiagnosticSeverity).includes(diagnostic.severity)) {
			throw new TypeError("Unknown language diagnostic severity");
		}
		if (typeof diagnostic.message !== "string" || diagnostic.message.trim().length === 0) {
			throw new TypeError("Language diagnostic message must not be empty");
		}
		if (diagnostic.code !== undefined) assertDiagnosticCode(diagnostic.code);
		if (diagnostic.source !== undefined) assertIdentifier(diagnostic.source, "Language diagnostic source");
		return Object.freeze({
			range: diagnostic.range,
			severity: diagnostic.severity,
			message: diagnostic.message,
			...(diagnostic.code === undefined ? {} : { code: diagnostic.code }),
			...(diagnostic.source === undefined ? {} : { source: diagnostic.source }),
		});
	});
	return Object.freeze({ diagnostics: Object.freeze(diagnostics) });
}

function assertModelRange(model: TextModel, range: TextRange, owner: string): void {
	if (!(range instanceof TextRange)) throw new TypeError(`${owner} range must be a TextRange`);
	model.offsetAt(range.start);
	model.offsetAt(range.end);
}

function assertSnapshotRange(lines: readonly string[], range: TextRange, owner: string): void {
	if (!(range instanceof TextRange)) throw new TypeError(`${owner} range must be a TextRange`);
	assertSnapshotPosition(lines, range.start, owner);
	assertSnapshotPosition(lines, range.end, owner);
}

function assertSnapshotPosition(lines: readonly string[], position: TextPosition, owner: string): void {
	if (position.lineIndex >= lines.length || position.columnIndex > lines[position.lineIndex]!.length) {
		throw new RangeError(`${owner} range is outside its snapshot`);
	}
}

function assertIdentifier(value: unknown, owner: string): asserts value is string {
	if (typeof value !== "string" || value.length === 0 || value.trim() !== value) {
		throw new TypeError(`${owner} must be a non-empty trimmed string`);
	}
}

function assertDiagnosticCode(code: LanguageDiagnosticCode): void {
	if (typeof code === "string") {
		assertIdentifier(code, "Language diagnostic code");
		return;
	}
	if (typeof code !== "number" || !Number.isFinite(code)) {
		throw new TypeError("Language diagnostic code must be a finite number or non-empty string");
	}
}
