import { TextRange, type TextPosition, type TextSnapshot } from "../core/text.js";
import { VersionedLanguageResultStore } from "../languages/languageResultStore.js";
import { type TextModel } from "../model/textModel.js";
import { attachLanguageTokenResultDelta, getLanguageTokenResultDelta } from '../services/semanticTokensDto.js';

export interface LanguageToken {
	readonly range: TextRange;
	readonly tokenType: string;
	readonly modifiers: readonly string[];
	/** Embedded language selected by the grammar for this source range. */
	readonly languageId?: string;
	/** False when the grammar excludes this scope from structural bracket matching. */
	readonly balancedBrackets?: false;
	/** Optional syntax-theme presentation retained independently of the semantic token type. */
	readonly presentation?: LanguageTokenPresentation;
}

export interface LanguageTokenPresentation {
	readonly foreground?: string;
	readonly background?: string;
	readonly fontStyle?: readonly LanguageTokenFontStyle[];
}

export type LanguageTokenFontStyle = "italic" | "bold" | "underline" | "strikethrough";

export interface LanguageTokenResult {
	readonly tokens: readonly LanguageToken[];
}

export function createLanguageTokenStore(model: TextModel): VersionedLanguageResultStore<LanguageTokenResult> {
	return new VersionedLanguageResultStore(model, (value, currentModel) => normalizeLanguageTokenResult(
		value,
		range => assertModelRange(currentModel, range, "Language token"),
		true,
	));
}

export function createLanguageTokenSnapshotNormalizer(snapshot: TextSnapshot): (value: LanguageTokenResult) => LanguageTokenResult {
	const lines = snapshot.getText().split("\n");
	return value => normalizeLanguageTokenResult(value, range => assertSnapshotRange(lines, range, "Language token"), false);
}

function normalizeLanguageTokenResult(value: LanguageTokenResult, validateRange: (range: TextRange) => void, preserveDelta: boolean): LanguageTokenResult {
	if (typeof value !== "object" || value === null || !Array.isArray(value.tokens)) {
		throw new TypeError("Language token result must contain a tokens array");
	}
	const delta = preserveDelta ? getLanguageTokenResultDelta(value) : undefined;
	const tokens: LanguageToken[] = [];
	let previousEnd: TextPosition | undefined;
	for (const token of value.tokens) {
		if (typeof token !== "object" || token === null) {
			throw new TypeError("Language token must be an object");
		}
		validateRange(token.range);
		if (token.range.empty) {
			throw new RangeError("Language token range must not be empty");
		}
		if (token.range.start.lineIndex !== token.range.end.lineIndex) {
			throw new RangeError("Language token range must stay on one line");
		}
		if (previousEnd && previousEnd.compareTo(token.range.start) > 0) {
			throw new RangeError("Language tokens must be sorted and non-overlapping");
		}
		assertIdentifier(token.tokenType, "Language token type");
		if (!Array.isArray(token.modifiers)) {
			throw new TypeError("Language token modifiers must be an array");
		}
		const modifiers = token.modifiers.map((modifier: unknown) => {
			assertIdentifier(modifier, "Language token modifier");
			return modifier;
		});
		if (new Set(modifiers).size !== modifiers.length) {
			throw new RangeError("Language token modifiers must be unique");
		}
		const languageId = token.languageId === undefined ? undefined : normalizedIdentifier(token.languageId, "Language token embedded language ID");
		if (token.balancedBrackets !== undefined && token.balancedBrackets !== false) throw new TypeError("Language token balanced-bracket metadata can only exclude a range");
		const presentation = token.presentation === undefined ? undefined : normalizePresentation(token.presentation);
		tokens.push(Object.freeze({
			range: token.range,
			tokenType: token.tokenType,
			modifiers: Object.freeze(modifiers),
			...(languageId === undefined ? {} : { languageId }),
			...(token.balancedBrackets === false ? { balancedBrackets: false as const } : {}),
			...(presentation === undefined ? {} : { presentation }),
		}));
		previousEnd = token.range.end;
	}
	const result = Object.freeze({ tokens: Object.freeze(tokens) });
	return delta ? attachLanguageTokenResultDelta(result, delta) : result;
}

function normalizePresentation(value: LanguageTokenPresentation): LanguageTokenPresentation {
	if (typeof value !== "object" || value === null) throw new TypeError("Language token presentation must be an object");
	const foreground = value.foreground === undefined ? undefined : normalizedColor(value.foreground, "foreground");
	const background = value.background === undefined ? undefined : normalizedColor(value.background, "background");
	let fontStyle: readonly LanguageTokenFontStyle[] | undefined;
	if (value.fontStyle !== undefined) {
		if (!Array.isArray(value.fontStyle)) throw new TypeError("Language token font style must be an array");
		const styles = value.fontStyle.map(style => {
			if (style !== "italic" && style !== "bold" && style !== "underline" && style !== "strikethrough") throw new TypeError(`Unsupported language token font style '${String(style)}'`);
			return style;
		});
		if (new Set(styles).size !== styles.length) throw new RangeError("Language token font styles must be unique");
		fontStyle = Object.freeze(styles);
	}
	return Object.freeze({ ...(foreground === undefined ? {} : { foreground }), ...(background === undefined ? {} : { background }), ...(fontStyle === undefined ? {} : { fontStyle }) });
}

function normalizedColor(value: unknown, kind: string): string {
	if (typeof value !== "string" || !/^#[0-9a-f]{3,4}(?:[0-9a-f]{3,4})?$/iu.test(value)) throw new TypeError(`Language token ${kind} must be a hexadecimal color`);
	return value;
}

function normalizedIdentifier(value: unknown, owner: string): string {
	assertIdentifier(value, owner);
	return value;
}

function assertModelRange(model: TextModel, range: TextRange, owner: string): void {
	if (!(range instanceof TextRange)) {
		throw new TypeError(`${owner} range must be a TextRange`);
	}
	model.offsetAt(range.start);
	model.offsetAt(range.end);
}

function assertSnapshotRange(lines: readonly string[], range: TextRange, owner: string): void {
	if (!(range instanceof TextRange)) {
		throw new TypeError(`${owner} range must be a TextRange`);
	}
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

