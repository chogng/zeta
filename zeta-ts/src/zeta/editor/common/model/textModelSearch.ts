import { escapeRegExpCharacters } from "../../../base/common/strings.js";
import { TextPosition } from "../core/text.js";
import { TextRange } from "../core/text.js";
import type { TextModel } from "./textModel.js";

const DEFAULT_RESULT_LIMIT = 999;
const MAX_RESULT_LIMIT = 100_000;
const MAX_PATTERN_LENGTH = 4_096;

/** Selects whether a search pattern is interpreted literally or as a JavaScript regular expression. */
export enum TextSearchPatternKind {
	Literal = "literal",
	RegularExpression = "regularExpression",
}

/** One caller-readable document search request. */
export interface TextSearchQuery {
	readonly pattern: string;
	readonly patternKind?: TextSearchPatternKind;
	readonly matchCase?: boolean;
	readonly wholeWord?: boolean;
}

/** Bounds one search without changing its matching semantics. */
export interface TextSearchOptions {
	readonly range?: TextRange;
	readonly resultLimit?: number;
}

/** One immutable match in the current TextModel version. */
export interface TextSearchMatch {
	readonly modelVersion: number;
	readonly range: TextRange;
	readonly text: string;
	readonly captures: readonly (string | undefined)[];
	readonly namedCaptures: Readonly<Record<string, string | undefined>>;
}

/** Reports a malformed or unsupported search request before scanning text. */
export class TextSearchQueryError extends Error {
	constructor(message: string, options?: ErrorOptions) {
		super(message, options);
		this.name = "TextSearchQueryError";
	}
}

/**
 * Finds ordered, non-overlapping matches in one synchronous Stanza TextModel snapshot.
 *
 * Offsets and ranges use UTF-16 code units. Empty regular-expression matches are supported and
 * always advance by one Unicode code point so a global search cannot loop forever.
 */
export function findTextMatches(model: TextModel, query: TextSearchQuery, options: TextSearchOptions = {}): readonly TextSearchMatch[] {
	validateQuery(query);
	const resultLimit = readResultLimit(options.resultLimit);
	if (query.pattern.length === 0 || resultLimit === 0) return Object.freeze([]);

	const snapshot = model.createSnapshot();
	const startOffset = options.range ? model.offsetAt(options.range.start) : 0;
	const endOffset = options.range ? model.offsetAt(options.range.end) : snapshot.length;
	const text = snapshot.getTextBetweenOffsets(startOffset, endOffset);
	const expression = compileQuery(query);
	const matches: TextSearchMatch[] = [];

	let match: RegExpExecArray | null;
	while (matches.length < resultLimit && (match = expression.exec(text))) {
		const relativeStart = match.index;
		const relativeEnd = relativeStart + match[0].length;
		if (!query.wholeWord || isWholeWordMatch(text, relativeStart, relativeEnd)) {
			matches.push(Object.freeze({
				modelVersion: snapshot.version,
				range: TextRange.from(
					model.positionAt(startOffset + relativeStart),
					model.positionAt(startOffset + relativeEnd),
				),
				text: match[0],
				captures: Object.freeze(match.slice(1)),
				namedCaptures: Object.freeze({ ...match.groups }),
			}));
		}
		if (match[0].length === 0) {
			if (expression.lastIndex >= text.length) break;
			expression.lastIndex = nextCodePointOffset(text, expression.lastIndex);
		}
	}

	return Object.freeze(matches);
}

/** Finds the first match at or after `from`, optionally wrapping once at the document end. */
export function findNextTextMatch(model: TextModel, query: TextSearchQuery, from: TextPosition, wrap = true): TextSearchMatch | undefined {
	const documentEnd = model.positionAt(model.createSnapshot().length);
	const following = findTextMatches(model, query, {
		range: TextRange.from(from, documentEnd),
		resultLimit: 1,
	})[0];
	if (following || !wrap || model.offsetAt(from) === 0) return following;
	return findTextMatches(model, query, {
		range: TextRange.from(TextPosition.at(0, 0), from),
		resultLimit: 1,
	})[0];
}

function validateQuery(query: TextSearchQuery): void {
	if (!query || typeof query !== "object") throw new TypeError("Text search requires a query");
	if (typeof query.pattern !== "string") throw new TypeError("Text search pattern must be a string");
	if (query.pattern.length > MAX_PATTERN_LENGTH) {
		throw new TextSearchQueryError(`Text search pattern must not exceed ${MAX_PATTERN_LENGTH} UTF-16 units`);
	}
	if (query.patternKind !== undefined && !Object.values(TextSearchPatternKind).includes(query.patternKind)) {
		throw new TextSearchQueryError(`Unsupported text search pattern kind: ${String(query.patternKind)}`);
	}
}

function readResultLimit(value: number | undefined): number {
	const limit = value ?? DEFAULT_RESULT_LIMIT;
	if (!Number.isSafeInteger(limit) || limit < 0 || limit > MAX_RESULT_LIMIT) {
		throw new RangeError(`Text search resultLimit must be an integer from 0 through ${MAX_RESULT_LIMIT}`);
	}
	return limit;
}

function compileQuery(query: TextSearchQuery): RegExp {
	const source = query.patternKind === TextSearchPatternKind.RegularExpression
		? query.pattern
		: escapeRegExpCharacters(query.pattern);
	try {
		return new RegExp(source, `gmu${query.matchCase ? "" : "i"}`);
	} catch (error) {
		throw new TextSearchQueryError("Text search regular expression is invalid", { cause: error });
	}
}

function isWholeWordMatch(text: string, start: number, end: number): boolean {
	const first = codePointAt(text, start);
	const last = codePointBefore(text, end);
	const before = codePointBefore(text, start);
	const after = codePointAt(text, end);
	return !(isWordCharacter(first) && isWordCharacter(before)) &&
		!(isWordCharacter(last) && isWordCharacter(after));
}

function isWordCharacter(value: string | undefined): boolean {
	return value !== undefined && /^[\p{L}\p{M}\p{N}_]$/u.test(value);
}

function codePointAt(text: string, offset: number): string | undefined {
	if (offset < 0 || offset >= text.length) return undefined;
	const value = text.codePointAt(offset);
	return value === undefined ? undefined : String.fromCodePoint(value);
}

function codePointBefore(text: string, offset: number): string | undefined {
	if (offset <= 0 || offset > text.length) return undefined;
	let start = offset - 1;
	const trailing = text.charCodeAt(start);
	if (trailing >= 0xdc00 && trailing <= 0xdfff && start > 0) {
		const leading = text.charCodeAt(start - 1);
		if (leading >= 0xd800 && leading <= 0xdbff) start -= 1;
	}
	return text.slice(start, offset);
}

function nextCodePointOffset(text: string, offset: number): number {
	const value = text.codePointAt(offset);
	return value === undefined ? text.length : offset + (value > 0xffff ? 2 : 1);
}
