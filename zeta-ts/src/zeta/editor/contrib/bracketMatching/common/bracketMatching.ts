import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import { type LanguageStructuralBracketSource } from "../../../common/languages/languageLexicalContext.js";
import { LanguageLexicalContextIndex } from "../../../common/languages/languageLexicalContext.js";
import { type LanguageConfigurationSource } from "../../../common/languages/languageConfiguration.js";
import { type LanguageLexicalBracketEvent } from "../../../common/languages/languageLexicalLineScanner.js";
import { TextPosition, TextRange } from "../../../common/core/text.js";
import { type TextModel } from "../../../common/model/textModel.js";

export interface LanguageBracketMatch {
	readonly opening: TextRange;
	readonly closing: TextRange;
}

export interface LanguageBracketMatcherOptions {
	readonly maxScanLineCount?: number;
}

interface BracketLocation {
	readonly lineIndex: number;
	readonly event: LanguageLexicalBracketEvent;
}

/** Finds configured structural bracket pairs while excluding lexical string/comment spans. */
export class LanguageBracketMatcher extends DisposableOwner {
	readonly textModel: TextModel;
	private readonly maxScanLineCount: number;
	private disposed = false;

	constructor(
		textModel: TextModel,
		brackets: LanguageStructuralBracketSource,
		options?: LanguageBracketMatcherOptions,
	);
	constructor(
		textModel: TextModel,
		languageId: string,
		configurations: LanguageConfigurationSource,
		options?: LanguageBracketMatcherOptions,
	);
	constructor(
		textModel: TextModel,
		bracketsOrLanguageId: LanguageStructuralBracketSource | string,
		configurationsOrOptions: LanguageConfigurationSource | LanguageBracketMatcherOptions = {},
		legacyOptions: LanguageBracketMatcherOptions = {},
	) {
		super();
		try {
			this.textModel = textModel;
			const brackets = typeof bracketsOrLanguageId === "string" ? this.own(new LanguageLexicalContextIndex(textModel, bracketsOrLanguageId, configurationsOrOptions as LanguageConfigurationSource)) : bracketsOrLanguageId;
			const options = typeof bracketsOrLanguageId === "string" ? legacyOptions : configurationsOrOptions as LanguageBracketMatcherOptions;
			this.brackets = brackets;
			if (brackets.textModel !== textModel) throw new TypeError("Language bracket matcher requires a structural source for its text model");
			this.maxScanLineCount = readMaxScanLineCount(options.maxScanLineCount);
			this.defer(() => {
				this.disposed = true;
			});
		} catch (error) {
			this.dispose();
			throw error;
		}
	}

	findMatch(position: TextPosition): LanguageBracketMatch | undefined {
		this.ensureAlive();
		this.textModel.offsetAt(position);
		const candidate = this.findCandidate(position);
		if (!candidate) return undefined;
		return candidate.event.action === "open"
			? this.findClosingMatch(candidate)
			: this.findOpeningMatch(candidate);
	}

	private findCandidate(position: TextPosition): BracketLocation | undefined {
		const events = this.bracketEventsAt(position.lineIndex);
		const contained = events.find(({ event }) => event.startColumn <= position.columnIndex && position.columnIndex < event.endColumn);
		if (contained) return contained;
		for (let index = events.length - 1; index >= 0; index -= 1) {
			const event = events[index]!;
			if (event.event.endColumn === position.columnIndex) return event;
		}
		return undefined;
	}

	private findClosingMatch(candidate: BracketLocation): LanguageBracketMatch | undefined {
		const expectedClosers = [candidate.event.matchingToken];
		const finalLine = Math.min(
			this.textModel.lineCount - 1,
			candidate.lineIndex + this.maxScanLineCount - 1,
		);
		for (let lineIndex = candidate.lineIndex; lineIndex <= finalLine; lineIndex += 1) {
			const events = this.bracketEventsAt(lineIndex);
			const startIndex = lineIndex === candidate.lineIndex
				? events.findIndex(location => sameLocation(location, candidate)) + 1
				: 0;
			for (let index = startIndex; index < events.length; index += 1) {
				const current = events[index]!;
				if (current.event.action === "open") {
					expectedClosers.push(current.event.matchingToken);
				} else if (expectedClosers.at(-1) === current.event.token) {
					expectedClosers.pop();
					if (expectedClosers.length === 0) return match(candidate, current);
				}
			}
		}
		return undefined;
	}

	private findOpeningMatch(candidate: BracketLocation): LanguageBracketMatch | undefined {
		const expectedOpeners = [candidate.event.matchingToken];
		const firstLine = Math.max(0, candidate.lineIndex - this.maxScanLineCount + 1);
		for (let lineIndex = candidate.lineIndex; lineIndex >= firstLine; lineIndex -= 1) {
			const events = this.bracketEventsAt(lineIndex);
			const startIndex = lineIndex === candidate.lineIndex
				? events.findIndex(location => sameLocation(location, candidate)) - 1
				: events.length - 1;
			for (let index = startIndex; index >= 0; index -= 1) {
				const current = events[index]!;
				if (current.event.action === "close") {
					expectedOpeners.push(current.event.matchingToken);
				} else if (expectedOpeners.at(-1) === current.event.token) {
					expectedOpeners.pop();
					if (expectedOpeners.length === 0) return match(current, candidate);
				}
			}
		}
		return undefined;
	}

	private bracketEventsAt(lineIndex: number): readonly BracketLocation[] {
		return Object.freeze(this.brackets.getStructuralBracketEvents(lineIndex).map(event => Object.freeze({ lineIndex, event })));
	}

	private readonly brackets: LanguageStructuralBracketSource;

	private ensureAlive(): void {
		if (this.disposed) throw new ReferenceError("Language bracket matcher is already disposed");
	}
}

function match(opening: BracketLocation, closing: BracketLocation): LanguageBracketMatch {
	return Object.freeze({
		opening: TextRange.from(
			TextPosition.at(opening.lineIndex, opening.event.startColumn),
			TextPosition.at(opening.lineIndex, opening.event.endColumn),
		),
		closing: TextRange.from(
			TextPosition.at(closing.lineIndex, closing.event.startColumn),
			TextPosition.at(closing.lineIndex, closing.event.endColumn),
		),
	});
}

function sameLocation(left: BracketLocation, right: BracketLocation): boolean {
	return left.lineIndex === right.lineIndex &&
		left.event.startColumn === right.event.startColumn &&
		left.event.endColumn === right.event.endColumn;
}

function readMaxScanLineCount(value: number | undefined): number {
	const result = value ?? 10_000;
	if (!Number.isSafeInteger(result) || result < 1) {
		throw new RangeError("Language bracket matcher max scan line count must be a positive safe integer");
	}
	return result;
}
