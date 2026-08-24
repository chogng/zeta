import { DisposableOwner } from "../../../base/common/lifecycle.js";
import { type LanguageConfigurationSource, type ResolvedLanguageConfiguration } from "./languageConfiguration.js";
import { assertLanguageId } from "./languageId.js";
import { createLanguageLexicalLineScanner } from "./languageLexicalConfiguration.js";
import { type LanguageLexicalBracketEvent, type LanguageLexicalLineResult, type LanguageLexicalLineScanner, type LanguageLexicalState } from "./languageLexicalLineScanner.js";
import { type TextModelChange, type TextPosition } from "../core/text.js";
import { type TextModel } from "../model/textModel.js";
import { type LanguageToken } from "../tokens/languageTokens.js";

export interface LanguageTokenizationSource {
	readonly textModel: TextModel;
	readonly modelVersion: number;
	getLineTokens(lineIndex: number): readonly LanguageToken[];
}

export interface LanguageLexicalContextSource {
	readonly textModel: TextModel;
	readonly languageId: string;
	getStructuralLineContent(lineIndex: number, startColumn?: number, endColumn?: number): string;
	getTokenTypeAt(position: TextPosition): string | undefined;
	getLanguageIdAt(position: TextPosition): string;
	supportsLanguageId(languageId: string): boolean;
}

/** Extends lexical context with bracket events whose columns remain in source text coordinates. */
export interface LanguageStructuralBracketSource extends LanguageLexicalContextSource {
	getStructuralBracketEvents(lineIndex: number): readonly LanguageLexicalBracketEvent[];
}

/**
 * Synchronous lexical context for one model/language identity.
 *
 * The index borrows its model and configuration source. It scans lazily to the
 * requested line and invalidates the affected suffix after model changes.
 */
export class LanguageLexicalContextIndex extends DisposableOwner implements LanguageStructuralBracketSource {
	private configuration: ResolvedLanguageConfiguration | undefined;
	private scanner: LanguageLexicalLineScanner | undefined;
	private bracketTokens: readonly string[] = Object.freeze([]);
	private lineResults: LanguageLexicalLineResult[] = [];

	constructor(readonly textModel: TextModel, readonly languageId: string, private readonly configurations: LanguageConfigurationSource) {
		super();
		assertLanguageId(languageId);
		if (!configurations || typeof configurations.getLanguageConfiguration !== "function") {
			this.dispose();
			throw new TypeError("Language lexical context requires a configuration source");
		}
		this.own(textModel.onDidChange(change => this.acceptModelChange(change)));
		this.defer(() => {
			this.configuration = undefined;
			this.scanner = undefined;
			this.bracketTokens = Object.freeze([]);
			this.lineResults = [];
		});
	}

	getStructuralLineContent(lineIndex: number, startColumn = 0, endColumn?: number): string {
		this.assertNotDisposed();
		assertLineIndex(this.textModel, lineIndex);
		const line = this.textModel.getLineContent(lineIndex);
		const resolvedEnd = endColumn ?? line.length;
		assertColumnRange(line, startColumn, resolvedEnd);
		const result = this.ensureLine(lineIndex);
		let content = "";
		let column = startColumn;
		for (const token of result.tokens) {
			if (token.endColumn <= startColumn || token.startColumn >= resolvedEnd) continue;
			const tokenStart = Math.max(startColumn, token.startColumn);
			const tokenEnd = Math.min(resolvedEnd, token.endColumn);
			if (column < tokenStart) content += line.slice(column, tokenStart);
			const tokenText = line.slice(tokenStart, tokenEnd);
			content += token.tokenType === "string" || token.tokenType === "comment" || token.tokenType === "regexp"
				? removeTokens(tokenText, this.bracketTokens)
				: tokenText;
			column = tokenEnd;
		}
		if (column < resolvedEnd) content += line.slice(column, resolvedEnd);
		return content;
	}

	getTokenTypeAt(position: TextPosition): string | undefined {
		this.assertNotDisposed();
		assertLineIndex(this.textModel, position.lineIndex);
		const line = this.textModel.getLineContent(position.lineIndex);
		assertColumnRange(line, position.columnIndex, position.columnIndex);
		const result = this.ensureLine(position.lineIndex);
		const containing = result.tokens.find(token => token.startColumn <= position.columnIndex && position.columnIndex < token.endColumn);
		if (containing) return containing.tokenType;
		if (position.columnIndex === line.length && result.outputState === "blockComment") return "comment";
		if (position.columnIndex === line.length && result.outputState !== "normal" && result.outputState !== "blockComment") return "string";
		const last = result.tokens.at(-1);
		if (position.columnIndex !== line.length || last?.endColumn !== line.length) return undefined;
		const lineComment = this.configuration!.comments.lineComment;
		if (last.tokenType === "comment" && result.inputState !== "blockComment" && lineComment && line.startsWith(lineComment, last.startColumn)) {
			return "comment";
		}
		if (last.tokenType === "string" && result.events.some(event => event.kind === "diagnostic" && event.endColumn === line.length)) {
			return "string";
		}
		return undefined;
	}

	getLanguageIdAt(position: TextPosition): string {
		this.textModel.offsetAt(position);
		return this.languageId;
	}

	supportsLanguageId(languageId: string): boolean {
		return languageId === this.languageId;
	}

	getStructuralBracketEvents(lineIndex: number): readonly LanguageLexicalBracketEvent[] {
		this.assertNotDisposed();
		assertLineIndex(this.textModel, lineIndex);
		return Object.freeze(this.ensureLine(lineIndex).events.flatMap(event => event.kind === "bracket" ? [event] : []));
	}

	private ensureLine(lineIndex: number): LanguageLexicalLineResult {
		const configuration = this.configurations.getLanguageConfiguration(this.languageId);
		if (configuration !== this.configuration) {
			this.configuration = configuration;
			this.scanner = createLanguageLexicalLineScanner(this.languageId, configuration);
			this.bracketTokens = structuralBracketTokens(configuration);
			this.lineResults = [];
		}
		let state: LanguageLexicalState = this.lineResults.at(-1)?.outputState ?? "normal";
		while (this.lineResults.length <= lineIndex) {
			const currentLine = this.lineResults.length;
			const result = this.scanner!.scan(this.textModel.getLineContent(currentLine), state);
			this.lineResults.push(result);
			state = result.outputState;
		}
		return this.lineResults[lineIndex]!;
	}

	private acceptModelChange(change: TextModelChange): void {
		const firstChangedLine = Math.min(...change.changes.map(contentChange => contentChange.range.start.lineIndex));
		this.lineResults.length = Math.min(this.lineResults.length, firstChangedLine);
	}

}

/** Uses grammar tokens when current and falls back to the deterministic lexical index. */
export class TokenAwareLanguageLexicalContext implements LanguageStructuralBracketSource {
	readonly textModel;
	readonly languageId;

	constructor(private readonly fallback: LanguageStructuralBracketSource, private readonly tokenization: LanguageTokenizationSource, private readonly configurations: LanguageConfigurationSource) {
		this.textModel = fallback.textModel;
		this.languageId = fallback.languageId;
		if (tokenization.textModel !== fallback.textModel) throw new TypeError("Token-aware lexical context requires one text model");
	}

	getStructuralLineContent(lineIndex: number, startColumn = 0, endColumn?: number): string {
		const line = this.textModel.getLineContent(lineIndex);
		const resolvedEnd = endColumn ?? line.length;
		if (this.tokenization.modelVersion !== this.textModel.version) return this.fallback.getStructuralLineContent(lineIndex, startColumn, resolvedEnd);
		let result = line.slice(startColumn, resolvedEnd);
		for (const token of this.tokenization.getLineTokens(lineIndex)) {
			if (!excludedFromStructure(token) || token.range.end.columnIndex <= startColumn || token.range.start.columnIndex >= resolvedEnd) continue;
			const from = Math.max(startColumn, token.range.start.columnIndex) - startColumn;
			const to = Math.min(resolvedEnd, token.range.end.columnIndex) - startColumn;
			result = result.slice(0, from) + " ".repeat(to - from) + result.slice(to);
		}
		return result;
	}

	getTokenTypeAt(position: TextPosition): string | undefined {
		const token = this.tokenAt(position);
		return token?.tokenType ?? this.fallback.getTokenTypeAt(position);
	}

	getLanguageIdAt(position: TextPosition): string {
		return this.tokenAt(position)?.languageId ?? this.languageId;
	}

	supportsLanguageId(_languageId: string): boolean {
		return true;
	}

	getStructuralBracketEvents(lineIndex: number): readonly LanguageLexicalBracketEvent[] {
		if (this.tokenization.modelVersion !== this.textModel.version) return this.fallback.getStructuralBracketEvents(lineIndex);
		const tokens = this.tokenization.getLineTokens(lineIndex);
		const embedded = tokens.filter(token => token.languageId !== undefined && token.languageId !== this.languageId);
		const events = this.fallback.getStructuralBracketEvents(lineIndex).filter(event => !tokens.some(token => (excludedFromStructure(token) || token.languageId !== undefined && token.languageId !== this.languageId) && contains(token, event.startColumn, event.endColumn)));
		const line = this.textModel.getLineContent(lineIndex);
		for (const token of embedded) {
			if (excludedFromStructure(token)) continue;
			const languageId = token.languageId!;
			const pairs = this.configurations.getLanguageConfiguration(languageId).brackets;
			const candidates = pairs.flatMap(pair => [{ token: pair.open, matchingToken: pair.close, action: "open" as const }, { token: pair.close, matchingToken: pair.open, action: "close" as const }]).sort((left, right) => right.token.length - left.token.length);
			let column = token.range.start.columnIndex;
			while (column < token.range.end.columnIndex) {
				const candidate = candidates.find(value => line.startsWith(value.token, column) && column + value.token.length <= token.range.end.columnIndex);
				if (!candidate) { column += 1; continue; }
				events.push(Object.freeze({ kind: "bracket", action: candidate.action, startColumn: column, endColumn: column + candidate.token.length, token: candidate.token, matchingToken: candidate.matchingToken }));
				column += candidate.token.length;
			}
		}
		return Object.freeze(events.sort((left, right) => left.startColumn - right.startColumn || left.endColumn - right.endColumn));
	}

	private tokenAt(position: TextPosition) {
		this.textModel.offsetAt(position);
		if (this.tokenization.modelVersion !== this.textModel.version) return undefined;
		return this.tokenization.getLineTokens(position.lineIndex).find(token => token.range.start.columnIndex <= position.columnIndex && position.columnIndex < token.range.end.columnIndex);
	}
}

function excludedFromStructure(token: LanguageToken): boolean {
	return token.balancedBrackets === false || token.tokenType === "string" || token.tokenType === "comment" || token.tokenType === "regexp";
}

function contains(token: LanguageToken, startColumn: number, endColumn: number): boolean {
	return token.range.start.columnIndex <= startColumn && token.range.end.columnIndex >= endColumn;
}

function structuralBracketTokens(configuration: ResolvedLanguageConfiguration): readonly string[] {
	return Object.freeze([...new Set(configuration.brackets.flatMap(pair => [pair.open, pair.close]))].sort((left, right) => right.length - left.length));
}

function removeTokens(text: string, tokens: readonly string[]): string {
	let result = text;
	for (const token of tokens) result = result.split(token).join("");
	return result;
}

function assertColumnRange(line: string, startColumn: number, endColumn: number): void {
	if (!Number.isSafeInteger(startColumn) || !Number.isSafeInteger(endColumn) || startColumn < 0 || endColumn < startColumn || endColumn > line.length) {
		throw new RangeError("Language lexical context columns must describe a valid line range");
	}
}

function assertLineIndex(model: TextModel, lineIndex: number): void {
	if (!Number.isSafeInteger(lineIndex) || lineIndex < 0 || lineIndex >= model.lineCount) {
		throw new RangeError("Language lexical context line is outside the text model");
	}
}
