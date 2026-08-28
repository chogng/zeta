import { type TextRange } from '../../core/text.js';

export interface InplaceReplaceResult {
	readonly range: TextRange;
	readonly value: string;
}

/** Cycles numeric and well-known textual values at one selection or word range. */
export class BasicInplaceReplace {
	public static readonly instance = new BasicInplaceReplace();

	public navigateValueSet(selectionRange: TextRange, selectionText: string, wordRange: TextRange | undefined, word: string | undefined, up: boolean): InplaceReplaceResult | undefined {
		const selectionValue = this.navigateValue(selectionText, up);
		if (selectionValue !== undefined) return Object.freeze({ range: selectionRange, value: selectionValue });
		if (!wordRange || word === undefined) return undefined;
		const wordValue = this.navigateValue(word, up);
		return wordValue === undefined ? undefined : Object.freeze({ range: wordRange, value: wordValue });
	}

	private navigateValue(value: string, up: boolean): string | undefined {
		const numeric = this.navigateNumber(value, up);
		if (numeric !== undefined) return numeric;
		for (const values of VALUE_SETS) {
			const index = values.indexOf(value);
			if (index < 0) continue;
			return values[(index + (up ? 1 : -1) + values.length) % values.length];
		}
		return undefined;
	}

	private navigateNumber(value: string, up: boolean): string | undefined {
		const number = Number(value);
		if (!Number.isFinite(number) || number !== Number.parseFloat(value)) return undefined;
		if (number === 0 && !up) return undefined;
		const decimalIndex = value.lastIndexOf('.');
		const precision = 10 ** (decimalIndex < 0 ? 0 : value.length - decimalIndex - 1);
		return String((Math.floor(number * precision) + (up ? precision : -precision)) / precision);
	}
}

const VALUE_SETS: readonly (readonly string[])[] = Object.freeze([
	Object.freeze(['true', 'false']),
	Object.freeze(['True', 'False']),
	Object.freeze(['Private', 'Public', 'Friend', 'ReadOnly', 'Partial', 'Protected', 'WriteOnly']),
	Object.freeze(['public', 'protected', 'private']),
]);
