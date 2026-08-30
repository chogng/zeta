import * as strings from '../../base/common/strings.js';
import { type TextDirection } from './model.js';
import { type IViewLineTokens } from './tokens/lineTokens.js';
import { type InlineDecoration } from './viewModel/inlineDecorations.js';

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
