export const USUAL_WORD_SEPARATORS = '`~!@#$%^&*()-=+[{]}\\|;:\'",.<>/?';

/** A word result expressed with one-based editor columns. */
export interface IWordAtPosition {
	readonly word: string;
	readonly startColumn: number;
	readonly endColumn: number;
}

export interface IGetWordAtTextConfig {
	readonly maxLen: number;
	readonly windowSize: number;
	readonly timeBudget: number;
}

export const DEFAULT_WORD_REGEXP = createWordRegExp();

let defaultConfig: IGetWordAtTextConfig = { maxLen: 1000, windowSize: 15, timeBudget: 150 };

export function setDefaultGetWordAtTextConfig(value: IGetWordAtTextConfig): () => void {
	const previous = defaultConfig;
	defaultConfig = validateConfig(value);
	return () => { defaultConfig = previous; };
}

export function ensureValidWordDefinition(wordDefinition?: RegExp | null): RegExp {
	if (!wordDefinition) return DEFAULT_WORD_REGEXP;
	const flags = wordDefinition.flags.includes("g") ? wordDefinition.flags : `${wordDefinition.flags}g`;
	const result = wordDefinition.flags.includes("g") ? wordDefinition : new RegExp(wordDefinition.source, flags);
	result.lastIndex = 0;
	return result;
}

/** Finds the word-like regex match enclosing a one-based editor column. */
export function getWordAtText(column: number, wordDefinition: RegExp, text: string, textOffset = 0, config = defaultConfig): IWordAtPosition | null {
	validateConfig(config);
	if (!Number.isSafeInteger(column) || column < 1) throw new RangeError("column must be a positive safe integer");
	const regex = ensureValidWordDefinition(wordDefinition);
	if (text.length > config.maxLen) {
		const halfWindow = Math.floor(config.maxLen / 2);
		const start = Math.max(0, column - 1 - halfWindow);
		const end = Math.min(text.length, column - 1 + halfWindow);
		return getWordAtText(column - start, regex, text.slice(start, end), textOffset + start, config);
	}
	const probe = column - 1 - textOffset;
	const startedAt = Date.now();
	let match: RegExpExecArray | null;
	while ((match = regex.exec(text))) {
		if (Date.now() - startedAt >= config.timeBudget) break;
		const start = match.index;
		const end = start + match[0].length;
		if (match[0].length > 0 && start <= probe && probe < end) {
			regex.lastIndex = 0;
			return { word: match[0], startColumn: textOffset + start + 1, endColumn: textOffset + end + 1 };
		}
		if (match[0].length === 0) regex.lastIndex += 1;
	}
	regex.lastIndex = 0;
	return null;
}

function createWordRegExp(allowInWords = ""): RegExp {
	let source = "(-?\\d*\\.\\d\\w*)|([^";
	for (const separator of USUAL_WORD_SEPARATORS) {
		if (!allowInWords.includes(separator)) source += `\\${separator}`;
	}
	return new RegExp(`${source}\\s]+)`, "g");
}

function validateConfig(config: IGetWordAtTextConfig): IGetWordAtTextConfig {
	if (!Number.isSafeInteger(config.maxLen) || config.maxLen < 1 || !Number.isSafeInteger(config.windowSize) || config.windowSize < 1 || !Number.isSafeInteger(config.timeBudget) || config.timeBudget < 1) {
		throw new RangeError("Invalid word lookup configuration");
	}
	return config;
}
