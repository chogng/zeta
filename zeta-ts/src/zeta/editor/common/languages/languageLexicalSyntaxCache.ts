import { commonPrefixLength, commonSuffixLength } from "../../../base/common/arrays.js";
import { type LanguageWorkerDocumentSynchronization } from '../services/textModelSync/textModelSync.protocol.js';
import { createBuiltinLanguageConfigurationSource } from "./languageBuiltinConfigurations.js";
import { createLanguageLexicalLineScanner } from "./languageLexicalConfiguration.js";
import { LanguageLexicalLineScanner, type LanguageLexicalLineResult, type LanguageLexicalMultilineEvent, type LanguageLexicalState } from "./languageLexicalLineScanner.js";
import { LanguageDiagnosticSeverity, type LanguageDiagnostic, type LanguageDiagnosticResult, type LanguageToken, type LanguageTokenResult } from "./languageResults.js";
import { TextPosition, TextRange, type TextSnapshot } from "../core/text.js";

export interface LanguageLexicalCacheUpdate {
	readonly modelVersion: number;
	readonly kind: "full" | "incremental";
	readonly scannedLineCount: number;
	readonly reusedLineCount: number;
}

export type LanguageLexicalCacheUpdateObserver = (update: LanguageLexicalCacheUpdate) => void;

export interface LanguageLexicalSyntaxCacheOptions {
	readonly scanner?: LanguageLexicalLineScanner;
	readonly onDidUpdate?: LanguageLexicalCacheUpdateObserver;
}

interface LanguageLexicalDocumentSyntax {
	readonly version: number;
	readonly lines: readonly string[];
	readonly lineResults: readonly LanguageLexicalLineResult[];
	readonly tokens: LanguageTokenResult;
	readonly diagnostics: LanguageDiagnosticResult;
}

interface OpenPosition {
	readonly token: string;
	readonly matchingToken: string;
	readonly position: TextPosition;
	readonly endColumn: number;
}

/** Versioned line cache shared by lexical token and diagnostic syntax lanes. */
export class LanguageLexicalSyntaxCache {
	private syntax: LanguageLexicalDocumentSyntax | undefined;
	private readonly scanner: LanguageLexicalLineScanner;
	private readonly onDidUpdate: LanguageLexicalCacheUpdateObserver | undefined;

	constructor(options: LanguageLexicalSyntaxCacheOptions = {}) {
		if (typeof options !== "object" || options === null) {
			throw new TypeError("Language lexical cache options must be an object");
		}
		this.scanner = options.scanner ?? DEFAULT_SCANNER;
		this.onDidUpdate = options.onDidUpdate;
		if (!(this.scanner instanceof LanguageLexicalLineScanner)) {
			throw new TypeError("Language lexical cache requires a line scanner");
		}
		if (this.onDidUpdate !== undefined && typeof this.onDidUpdate !== "function") {
			throw new TypeError("Language lexical cache update observer must be a function");
		}
	}

	getTokens(snapshot: TextSnapshot, signal: AbortSignal): LanguageTokenResult {
		return this.ensure(snapshot, signal).tokens;
	}

	getDiagnostics(snapshot: TextSnapshot, signal: AbortSignal): LanguageDiagnosticResult {
		return this.ensure(snapshot, signal).diagnostics;
	}

	synchronizeDocument(synchronization: LanguageWorkerDocumentSynchronization): void {
		if (synchronization.snapshot.version !== synchronization.modelVersion) {
			throw new Error("Language lexical synchronization snapshot version is inconsistent");
		}
		if (!this.syntax) return;
		if (this.syntax.version !== synchronization.previousVersion) {
			this.syntax = undefined;
			return;
		}
		this.syntax = this.update(synchronization.snapshot, undefined, "incremental");
	}

	private ensure(snapshot: TextSnapshot, signal: AbortSignal): LanguageLexicalDocumentSyntax {
		if (this.syntax?.version === snapshot.version) return this.syntax;
		const kind = this.syntax ? "incremental" : "full";
		this.syntax = this.update(snapshot, signal, kind);
		return this.syntax;
	}

	private update(snapshot: TextSnapshot, signal: AbortSignal | undefined, kind: LanguageLexicalCacheUpdate["kind"]): LanguageLexicalDocumentSyntax {
		const text = snapshot.getText();
		const lines = Object.freeze(text.split("\n"));
		if (text.length !== snapshot.length || lines.length !== snapshot.lineCount) {
			throw new Error("Language lexical snapshot metadata is inconsistent");
		}
		const previous = kind === "incremental" ? this.syntax : undefined;
		const scanned = previous
			? updateLines(this.scanner, previous.lines, previous.lineResults, lines, signal)
			: scanAllLines(this.scanner, lines, signal);
		const results = aggregateResults(scanned.lineResults);
		const syntax = Object.freeze({
			version: snapshot.version,
			lines,
			lineResults: scanned.lineResults,
			tokens: results.tokens,
			diagnostics: results.diagnostics,
		});
		this.onDidUpdate?.(Object.freeze({
			modelVersion: snapshot.version,
			kind: previous ? "incremental" : "full",
			scannedLineCount: scanned.scannedLineCount,
			reusedLineCount: lines.length - scanned.scannedLineCount,
		}));
		return syntax;
	}
}

function scanAllLines(scanner: LanguageLexicalLineScanner, lines: readonly string[], signal?: AbortSignal): { readonly lineResults: readonly LanguageLexicalLineResult[]; readonly scannedLineCount: number } {
	const lineResults: LanguageLexicalLineResult[] = [];
	let state: LanguageLexicalState = "normal";
	for (const line of lines) {
		signal?.throwIfAborted();
		const result = scanner.scan(line, state, signal);
		lineResults.push(result);
		state = result.outputState;
	}
	return { lineResults: Object.freeze(lineResults), scannedLineCount: lines.length };
}

function updateLines(scanner: LanguageLexicalLineScanner, previousLines: readonly string[], previousResults: readonly LanguageLexicalLineResult[], lines: readonly string[], signal?: AbortSignal): { readonly lineResults: readonly LanguageLexicalLineResult[]; readonly scannedLineCount: number } {
	const prefixLength = commonPrefixLength(previousLines, lines);
	const suffixLength = commonSuffixLength(previousLines, lines, prefixLength);
	const lineResults = previousResults.slice(0, prefixLength);
	const newSuffixStart = lines.length - suffixLength;
	const oldSuffixStart = previousLines.length - suffixLength;
	let state: LanguageLexicalState = lineResults.at(-1)?.outputState ?? "normal";
	let scannedLineCount = 0;
	for (let lineIndex = prefixLength; lineIndex < lines.length; lineIndex += 1) {
		signal?.throwIfAborted();
		if (lineIndex >= newSuffixStart) {
			const oldIndex = oldSuffixStart + lineIndex - newSuffixStart;
			const cached = previousResults[oldIndex]!;
			if (cached.inputState === state) {
				lineResults.push(...previousResults.slice(oldIndex));
				break;
			}
		}
		const result = scanner.scan(lines[lineIndex]!, state, signal);
		lineResults.push(result);
		state = result.outputState;
		scannedLineCount += 1;
	}
	return { lineResults: Object.freeze(lineResults), scannedLineCount };
}

function aggregateResults(lineResults: readonly LanguageLexicalLineResult[]): { readonly tokens: LanguageTokenResult; readonly diagnostics: LanguageDiagnosticResult } {
	const tokens: LanguageToken[] = [];
	const diagnostics: LanguageDiagnostic[] = [];
	const brackets: OpenPosition[] = [];
	let multiline: { readonly kind: LanguageLexicalMultilineEvent["lexicalKind"]; readonly position: TextPosition; readonly endColumn: number } | undefined;
	for (let lineIndex = 0; lineIndex < lineResults.length; lineIndex += 1) {
		const result = lineResults[lineIndex]!;
		for (const token of result.tokens) {
			tokens.push(Object.freeze({
				range: lineRange(lineIndex, token.startColumn, token.endColumn),
				tokenType: token.tokenType,
				modifiers: Object.freeze([]),
			}));
		}
		for (const event of result.events) {
			if (event.kind === "diagnostic") {
				diagnostics.push(diagnostic(lineRange(lineIndex, event.startColumn, event.endColumn), event.severity, event.message));
				continue;
			}
			if (event.kind === "multiline") {
				if (event.action === "open") {
					multiline = { kind: event.lexicalKind, position: TextPosition.at(lineIndex, event.columnIndex), endColumn: event.endColumn };
				} else {
					multiline = undefined;
				}
				continue;
			}
			if (event.action === "open") {
				brackets.push({
					token: event.token,
					matchingToken: event.matchingToken,
					position: TextPosition.at(lineIndex, event.startColumn),
					endColumn: event.endColumn,
				});
				continue;
			}
			const opener = brackets.at(-1);
			if (!opener || opener.matchingToken !== event.token) {
				diagnostics.push(diagnostic(lineRange(lineIndex, event.startColumn, event.endColumn), LanguageDiagnosticSeverity.Error, `Unexpected closing bracket '${event.token}'`));
			} else {
				brackets.pop();
			}
		}
	}
	if (multiline) {
		diagnostics.push(diagnostic(TextRange.from(multiline.position, TextPosition.at(multiline.position.lineIndex, multiline.endColumn)), LanguageDiagnosticSeverity.Warning, unterminatedMultilineMessage(multiline.kind)));
	}
	for (const opener of brackets) {
		diagnostics.push(diagnostic(TextRange.from(opener.position, TextPosition.at(opener.position.lineIndex, opener.endColumn)), LanguageDiagnosticSeverity.Warning, `Unclosed bracket '${opener.token}'`));
	}
	return {
		tokens: Object.freeze({ tokens: Object.freeze(tokens) }),
		diagnostics: Object.freeze({ diagnostics: Object.freeze(diagnostics) }),
	};
}

function unterminatedMultilineMessage(kind: LanguageLexicalMultilineEvent["lexicalKind"]): string {
	if (kind === "blockComment") return "Unterminated block comment";
	if (kind === "multilineString") return "Unterminated template literal";
	return "Unterminated raw string literal";
}

const defaultConfigurationSource = createBuiltinLanguageConfigurationSource();
const DEFAULT_SCANNER = createLanguageLexicalLineScanner("typescript", defaultConfigurationSource.getLanguageConfiguration("typescript"));

function lineRange(lineIndex: number, startColumn: number, endColumn: number): TextRange {
	return TextRange.from(TextPosition.at(lineIndex, startColumn), TextPosition.at(lineIndex, endColumn));
}

function diagnostic(range: TextRange, severity: LanguageDiagnosticSeverity, message: string): LanguageDiagnostic {
	return Object.freeze({ range, severity, message, source: "language.lexical" });
}
