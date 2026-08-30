import { Emitter, type Event } from "../../../base/common/event.js";
import { Disposable, toDisposable } from "../../../base/common/lifecycle.js";
import { type MergedLanguageConfigurationChangeEvent, type LanguageConfigurationSource, type MergedLanguageConfiguration } from "./ownedLanguageConfigurationContributions.js";
import { assertLanguageId } from "./languageId.js";
import { createLanguageLexicalLineScanner } from "./languageLexicalConfiguration.js";
import { type LanguageLexicalBracketEvent, type LanguageLexicalLineResult, type LanguageLexicalLineScanner, type LanguageLexicalState } from "./languageLexicalLineScanner.js";
import { type Position } from "../core/position.js";
import { type TextModelChange } from "../core/textChange.js";
import { type TextModel } from "../model/textModel.js";
import { type LanguageToken } from "../tokens/languageTokens.js";

export interface LanguageTokenizationSource {
	readonly textModel: TextModel;
	readonly modelVersion: number;
	readonly onDidChange: Event<void>;
	getLineTokens(lineIndex: number): readonly LanguageToken[];
}

export interface LanguageLexicalContextSource {
	readonly textModel: TextModel;
	readonly languageId: string;
	getStructuralLineContent(lineIndex: number, startColumn?: number, endColumn?: number): string;
	getTokenTypeAt(position: Position): string | undefined;
	getLanguageIdAt(position: Position): string;
	supportsLanguageId(languageId: string): boolean;
}

/** Extends lexical context with bracket events whose columns remain in source text coordinates. */
export interface LanguageStructuralBracketSource extends LanguageLexicalContextSource {
	readonly onDidChange: Event<void>;
	getStructuralBracketEvents(lineIndex: number): readonly LanguageLexicalBracketEvent[];
}

/**
 * Synchronous lexical context for one model/language identity.
 *
 * The index borrows its model and configuration source. It scans lazily to the
 * requested line and invalidates the affected suffix after model changes.
 */
export class LanguageLexicalContextIndex extends Disposable implements LanguageStructuralBracketSource {
	private readonly changeEmitter = this._register(new Emitter<void>());
	private configuration: MergedLanguageConfiguration | undefined;
	private scanner: LanguageLexicalLineScanner | undefined;
	private bracketTokens: readonly string[] = Object.freeze([]);
	private lineResults: LanguageLexicalLineResult[] = [];
	readonly onDidChange: Event<void> = this.changeEmitter.event;

	constructor(readonly textModel: TextModel, readonly languageId: string, private readonly configurations: LanguageConfigurationSource) {
		super();
		assertLanguageId(languageId);
		if (!configurations || typeof configurations.getLanguageConfiguration !== "function") {
			this.dispose();
			throw new TypeError("Language lexical context requires a configuration source");
		}
		this._register(textModel.onDidChangeContent(change => this.acceptModelChange(change)));
		if (configurations.onDidChange) {
			this._register(configurations.onDidChange(event => this.acceptConfigurationChange(event)));
		}
		this._register(toDisposable(() => {
			this.configuration = undefined;
			this.scanner = undefined;
			this.bracketTokens = Object.freeze([]);
			this.lineResults = [];
		}));
	}

	getStructuralLineContent(lineIndex: number, startColumn = 0, endColumn?: number): string {
		this.assertNotDisposed();
		assertLineIndex(this.textModel, lineIndex);
		const line = this.textModel.getLineContent((lineIndex) + 1);
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

	getTokenTypeAt(position: Position): string | undefined {
		this.assertNotDisposed();
		const lineIndex = position.lineNumber - 1;
		const columnIndex = position.column - 1;
		assertLineIndex(this.textModel, lineIndex);
		const line = this.textModel.getLineContent(position.lineNumber);
		assertColumnRange(line, columnIndex, columnIndex);
		const result = this.ensureLine(lineIndex);
		const containing = result.tokens.find(token => token.startColumn <= columnIndex && columnIndex < token.endColumn);
		if (containing) return containing.tokenType;
		if (columnIndex === line.length && result.outputState === "blockComment") return "comment";
		if (columnIndex === line.length && result.outputState !== "normal" && result.outputState !== "blockComment") return "string";
		const last = result.tokens.at(-1);
		if (columnIndex !== line.length || last?.endColumn !== line.length) return undefined;
		const lineComment = this.configuration!.comments.lineComment;
		if (last.tokenType === "comment" && result.inputState !== "blockComment" && lineComment && line.startsWith(lineComment, last.startColumn)) {
			return "comment";
		}
		if (last.tokenType === "string" && result.events.some(event => event.kind === "diagnostic" && event.endColumn === line.length)) {
			return "string";
		}
		return undefined;
	}

	getLanguageIdAt(position: Position): string {
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
			const result = this.scanner!.scan(this.textModel.getLineContent((currentLine) + 1), state);
			this.lineResults.push(result);
			state = result.outputState;
		}
		return this.lineResults[lineIndex]!;
	}

	private acceptModelChange(change: TextModelChange): void {
		const firstChangedLine = Math.min(...change.changes.map(contentChange => contentChange.range.startLineNumber - 1));
		this.lineResults.length = Math.min(this.lineResults.length, firstChangedLine);
		this.changeEmitter.fire();
	}

	private acceptConfigurationChange(event: MergedLanguageConfigurationChangeEvent): void {
		if (event.languageId !== this.languageId) return;
		this.configuration = undefined;
		this.scanner = undefined;
		this.bracketTokens = Object.freeze([]);
		this.lineResults = [];
		this.changeEmitter.fire();
	}

}

/** Uses grammar tokens when current and falls back to the deterministic lexical index. */
export class TokenAwareLanguageLexicalContext extends Disposable implements LanguageStructuralBracketSource {
	private readonly changeEmitter = this._register(new Emitter<void>());
	readonly textModel;
	readonly languageId;
	readonly onDidChange: Event<void> = this.changeEmitter.event;

	constructor(private readonly fallback: LanguageStructuralBracketSource, private readonly tokenization: LanguageTokenizationSource, private readonly configurations: LanguageConfigurationSource) {
		super();
		this.textModel = fallback.textModel;
		this.languageId = fallback.languageId;
		if (tokenization.textModel !== fallback.textModel) {
			this.dispose();
			throw new TypeError("Token-aware lexical context requires one text model");
		}
		this._register(fallback.onDidChange(() => this.changeEmitter.fire()));
		this._register(tokenization.onDidChange(() => this.changeEmitter.fire()));
	}

	getStructuralLineContent(lineIndex: number, startColumn = 0, endColumn?: number): string {
		this.assertNotDisposed();
		const line = this.textModel.getLineContent((lineIndex) + 1);
		const resolvedEnd = endColumn ?? line.length;
		if (this.tokenization.modelVersion !== this.textModel.version) return this.fallback.getStructuralLineContent(lineIndex, startColumn, resolvedEnd);
		let result = line.slice(startColumn, resolvedEnd);
		for (const token of this.tokenization.getLineTokens(lineIndex)) {
			if (!excludedFromStructure(token) || token.range.endColumn - 1 <= startColumn || token.range.startColumn - 1 >= resolvedEnd) continue;
			const from = Math.max(startColumn, token.range.startColumn - 1) - startColumn;
			const to = Math.min(resolvedEnd, token.range.endColumn - 1) - startColumn;
			result = result.slice(0, from) + " ".repeat(to - from) + result.slice(to);
		}
		return result;
	}

	getTokenTypeAt(position: Position): string | undefined {
		this.assertNotDisposed();
		const token = this.tokenAt(position);
		return token?.tokenType ?? this.fallback.getTokenTypeAt(position);
	}

	getLanguageIdAt(position: Position): string {
		this.assertNotDisposed();
		return this.tokenAt(position)?.languageId ?? this.languageId;
	}

	supportsLanguageId(_languageId: string): boolean {
		this.assertNotDisposed();
		return true;
	}

	getStructuralBracketEvents(lineIndex: number): readonly LanguageLexicalBracketEvent[] {
		this.assertNotDisposed();
		if (this.tokenization.modelVersion !== this.textModel.version) return this.fallback.getStructuralBracketEvents(lineIndex);
		const tokens = this.tokenization.getLineTokens(lineIndex);
		const embedded = tokens.filter(token => token.languageId !== undefined && token.languageId !== this.languageId);
		const events = this.fallback.getStructuralBracketEvents(lineIndex).filter(event => !tokens.some(token => (excludedFromStructure(token) || token.languageId !== undefined && token.languageId !== this.languageId) && contains(token, event.startColumn, event.endColumn)));
		const line = this.textModel.getLineContent((lineIndex) + 1);
		for (const token of embedded) {
			if (excludedFromStructure(token)) continue;
			const languageId = token.languageId!;
			const pairs = this.configurations.getLanguageConfiguration(languageId).brackets;
			const candidates = pairs.flatMap(pair => [{ token: pair.open, matchingToken: pair.close, action: "open" as const }, { token: pair.close, matchingToken: pair.open, action: "close" as const }]).sort((left, right) => right.token.length - left.token.length);
			let column = token.range.startColumn - 1;
			while (column < token.range.endColumn - 1) {
				const candidate = candidates.find(value => line.startsWith(value.token, column) && column + value.token.length <= token.range.endColumn - 1);
				if (!candidate) { column += 1; continue; }
				events.push(Object.freeze({ kind: "bracket", action: candidate.action, startColumn: column, endColumn: column + candidate.token.length, token: candidate.token, matchingToken: candidate.matchingToken }));
				column += candidate.token.length;
			}
		}
		return Object.freeze(events.sort((left, right) => left.startColumn - right.startColumn || left.endColumn - right.endColumn));
	}

	private tokenAt(position: Position) {
		this.textModel.offsetAt(position);
		if (this.tokenization.modelVersion !== this.textModel.version) return undefined;
		const columnIndex = position.column - 1;
		return this.tokenization.getLineTokens(position.lineNumber - 1).find(token => token.range.startColumn - 1 <= columnIndex && columnIndex < token.range.endColumn - 1);
	}
}

function excludedFromStructure(token: LanguageToken): boolean {
	return token.balancedBrackets === false || token.tokenType === "string" || token.tokenType === "comment" || token.tokenType === "regexp";
}

function contains(token: LanguageToken, startColumn: number, endColumn: number): boolean {
	return token.range.startColumn - 1 <= startColumn && token.range.endColumn - 1 >= endColumn;
}

function structuralBracketTokens(configuration: MergedLanguageConfiguration): readonly string[] {
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
