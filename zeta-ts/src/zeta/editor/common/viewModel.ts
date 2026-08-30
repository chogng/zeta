import * as strings from '../../base/common/strings.js';
import { type TextDirection } from './model.js';
import { type IViewLineTokens } from './tokens/lineTokens.js';
import { type InlineDecoration } from './viewModel/inlineDecorations.js';

export interface IEditorWhitespace {
	readonly id: string;
	readonly afterLineNumber: number;
	readonly height: number;
}

export interface IWhitespaceChangeAccessor {
	insertWhitespace(afterLineNumber: number, ordinal: number, heightInPx: number, minWidth: number): string;
	changeOneWhitespace(id: string, newAfterLineNumber: number, newHeight: number): void;
	removeWhitespace(id: string): void;
}

export interface ILineHeightChangeAccessor {
	insertOrChangeCustomLineHeight(decorationId: string, startLineNumber: number, endLineNumber: number, lineHeight: number): void;
	removeCustomLineHeight(decorationId: string): void;
}

export interface IPartialViewLinesViewportData {
	readonly bigNumbersDelta: number;
	readonly startLineNumber: number;
	readonly endLineNumber: number;
	readonly relativeVerticalOffset: number[];
	readonly centeredLineNumber: number;
	readonly completelyVisibleStartLineNumber: number;
	readonly completelyVisibleEndLineNumber: number;
	readonly lineHeight: number;
}

export interface IViewWhitespaceViewportData {
	readonly id: string;
	readonly afterLineNumber: number;
	readonly verticalOffset: number;
	readonly height: number;
}

export class ViewLineRenderingData {
	public readonly minColumn: number;
	public readonly maxColumn: number;
	public readonly content: string;
	public readonly continuesWithWrappedLine: boolean;
	public readonly containsRTL: boolean;
	public readonly isBasicASCII: boolean;
	public readonly tokens: IViewLineTokens;
	public readonly inlineDecorations: InlineDecoration[];
	public readonly tabSize: number;
	public readonly startVisibleColumn: number;
	public readonly textDirection: TextDirection;
	public readonly hasVariableFonts: boolean;

	constructor(
		minColumn: number,
		maxColumn: number,
		content: string,
		continuesWithWrappedLine: boolean,
		mightContainRTL: boolean,
		mightContainNonBasicASCII: boolean,
		tokens: IViewLineTokens,
		inlineDecorations: InlineDecoration[],
		tabSize: number,
		startVisibleColumn: number,
		textDirection: TextDirection,
		hasVariableFonts: boolean
	) {
		this.minColumn = minColumn;
		this.maxColumn = maxColumn;
		this.content = content;
		this.continuesWithWrappedLine = continuesWithWrappedLine;
		this.isBasicASCII = ViewLineRenderingData.isBasicASCII(content, mightContainNonBasicASCII);
		this.containsRTL = ViewLineRenderingData.containsRTL(content, this.isBasicASCII, mightContainRTL);
		this.tokens = tokens;
		this.inlineDecorations = inlineDecorations;
		this.tabSize = tabSize;
		this.startVisibleColumn = startVisibleColumn;
		this.textDirection = textDirection;
		this.hasVariableFonts = hasVariableFonts;
	}

	public static isBasicASCII(lineContent: string, mightContainNonBasicASCII: boolean): boolean {
		if (mightContainNonBasicASCII) {
			return strings.isBasicASCII(lineContent);
		}
		return true;
	}

	public static containsRTL(lineContent: string, isBasicASCII: boolean, mightContainRTL: boolean): boolean {
		if (!isBasicASCII && mightContainRTL) {
			return strings.containsRTL(lineContent);
		}
		return false;
	}
}
