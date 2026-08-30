import { type InjectedTextOptions } from './model.js';

export interface FixedWidthInjectedTextRange {
	readonly startOffset: number;
	readonly endOffset: number;
	readonly widthInEm: number;
}

/** One piece of view-only text attached to a model position. */
export class LineInjectedText {
	public static applyInjectedText(lineText: string, injectedTexts: readonly LineInjectedText[] | null): string {
		if (!injectedTexts || injectedTexts.length === 0) return lineText;
		let result = '';
		let lastOriginalOffset = 0;
		for (const injectedText of injectedTexts) {
			result += lineText.substring(lastOriginalOffset, injectedText.column - 1);
			lastOriginalOffset = injectedText.column - 1;
			result += injectedText.options.content;
		}
		return result + lineText.substring(lastOriginalOffset);
	}

	public static getFixedWidthInjectedTextRanges(injectedTexts: readonly LineInjectedText[] | null): FixedWidthInjectedTextRange[] {
		const result: FixedWidthInjectedTextRange[] = [];
		let injectedTextLength = 0;
		for (const injectedText of injectedTexts ?? []) {
			const length = injectedText.options.content.length;
			const startOffset = injectedText.column - 1 + injectedTextLength;
			const widthInEm = injectedText.options.widthInEm;
			if (widthInEm !== undefined) result.push({ startOffset, endOffset: startOffset + length, widthInEm });
			injectedTextLength += length;
		}
		return result;
	}

	constructor(
		public readonly ownerId: number,
		public readonly lineNumber: number,
		public readonly column: number,
		public readonly options: InjectedTextOptions,
		public readonly order: number,
	) {}

	public withText(text: string): LineInjectedText {
		return new LineInjectedText(this.ownerId, this.lineNumber, this.column, { ...this.options, content: text }, this.order);
	}
}

/** Describes a change to the language associated with a text model. */
export interface IModelLanguageChangedEvent {
	readonly oldLanguage: string;
	readonly newLanguage: string;
	readonly source: string;
}

export interface IModelOptionsChangedEvent {
	readonly tabSize: boolean;
	readonly indentSize: boolean;
	readonly insertSpaces: boolean;
	readonly trimAutoWhitespace: boolean;
}

/** Describes which editor decoration lanes need to be recomputed. */
export interface IModelDecorationsChangedEvent {
	readonly affectsMinimap: boolean;
	readonly affectsOverviewRuler: boolean;
	readonly affectsGlyphMargin: boolean;
	readonly affectsLineNumber: boolean;
}
