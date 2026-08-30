import { toDisposable } from '../../../base/common/lifecycle.js';
import { LinkedList } from '../../../base/common/linkedList.js';

export const USUAL_WORD_SEPARATORS = '`~!@#$%^&*()-=+[{]}\\|;:\'",.<>/?';

export interface IWordAtPosition {
	readonly word: string;
	readonly startColumn: number;
	readonly endColumn: number;
}

function createWordRegExp(allowInWords: string = ''): RegExp {
	let source = '(-?\\d*\\.\\d\\w*)|([^';
	for (const separator of USUAL_WORD_SEPARATORS) {
		if (allowInWords.indexOf(separator) >= 0) continue;
		source += '\\' + separator;
	}
	source += '\\s]+)';
	return new RegExp(source, 'g');
}

export const DEFAULT_WORD_REGEXP = createWordRegExp();

export function ensureValidWordDefinition(wordDefinition?: RegExp | null): RegExp {
	let result = DEFAULT_WORD_REGEXP;
	if (wordDefinition instanceof RegExp) {
		if (!wordDefinition.global) {
			let flags = 'g';
			if (wordDefinition.ignoreCase) flags += 'i';
			if (wordDefinition.multiline) flags += 'm';
			if (wordDefinition.unicode) flags += 'u';
			result = new RegExp(wordDefinition.source, flags);
		} else {
			result = wordDefinition;
		}
	}
	result.lastIndex = 0;
	return result;
}

export interface IGetWordAtTextConfig {
	maxLen: number;
	windowSize: number;
	timeBudget: number;
}

const defaultConfigs = new LinkedList<IGetWordAtTextConfig>();
defaultConfigs.unshift({ maxLen: 1000, windowSize: 15, timeBudget: 150 });

export function setDefaultGetWordAtTextConfig(value: IGetWordAtTextConfig) {
	return toDisposable(defaultConfigs.unshift(value));
}

export function getWordAtText(column: number, wordDefinition: RegExp, text: string, textOffset: number, config?: IGetWordAtTextConfig): IWordAtPosition | null {
	wordDefinition = ensureValidWordDefinition(wordDefinition);
	const activeConfig = config ?? defaultConfigs[Symbol.iterator]().next().value;
	if (!activeConfig) throw new Error('Word lookup requires a default configuration');

	if (text.length > activeConfig.maxLen) {
		let start = column - activeConfig.maxLen / 2;
		if (start < 0) start = 0;
		else textOffset += start;
		text = text.substring(start, column + activeConfig.maxLen / 2);
		return getWordAtText(column, wordDefinition, text, textOffset, activeConfig);
	}

	const startedAt = Date.now();
	const position = column - 1 - textOffset;
	let previousRegexIndex = -1;
	let match: RegExpExecArray | null = null;
	for (let index = 1; ; index += 1) {
		if (Date.now() - startedAt >= activeConfig.timeBudget) break;
		const regexIndex = position - activeConfig.windowSize * index;
		wordDefinition.lastIndex = Math.max(0, regexIndex);
		const currentMatch = findRegexMatchEnclosingPosition(wordDefinition, text, position, previousRegexIndex);
		if (!currentMatch && match) break;
		match = currentMatch;
		if (regexIndex <= 0) break;
		previousRegexIndex = regexIndex;
	}

	if (!match) return null;
	const result = {
		word: match[0],
		startColumn: textOffset + 1 + match.index,
		endColumn: textOffset + 1 + match.index + match[0].length,
	};
	wordDefinition.lastIndex = 0;
	return result;
}

function findRegexMatchEnclosingPosition(wordDefinition: RegExp, text: string, position: number, stopPosition: number): RegExpExecArray | null {
	let match: RegExpExecArray | null;
	while ((match = wordDefinition.exec(text))) {
		const matchIndex = match.index || 0;
		if (matchIndex <= position && wordDefinition.lastIndex >= position) return match;
		if (stopPosition > 0 && matchIndex > stopPosition) return null;
	}
	return null;
}
