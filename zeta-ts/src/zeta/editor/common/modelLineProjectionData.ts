import { WrappingIndent } from './config/editorOptions.js';
import { type FontInfo } from './config/fontInfo.js';
import { type InjectedTextOptions } from './model.js';
import { type LineInjectedText } from './textModelEvents.js';

/**
 * The model-to-view data produced for one logical line.
 *
 * Break offsets are UTF-16 offsets after injected text has been applied. The
 * wrapped indent is expressed in visible columns; browser rendering converts
 * it to pixels with the same measured font information used to wrap the line.
 */
export class ModelLineProjectionData {
	constructor(
		public injectionOffsets: number[] | null,
		public injectionOptions: InjectedTextOptions[] | null,
		public breakOffsets: number[],
		public breakOffsetsVisibleColumn: number[],
		public wrappedTextIndentLength: number,
	) {}

	public getOutputLineCount(): number {
		return this.breakOffsets.length;
	}

	public getMinOutputOffset(outputLineIndex: number): number {
		return outputLineIndex > 0 ? this.wrappedTextIndentLength : 0;
	}

	public getLineLength(outputLineIndex: number): number {
		const startOffset = outputLineIndex > 0 ? this.breakOffsets[outputLineIndex - 1]! : 0;
		const endOffset = this.breakOffsets[outputLineIndex]!;
		return endOffset - startOffset + (outputLineIndex > 0 ? this.wrappedTextIndentLength : 0);
	}

	public getMaxOutputOffset(outputLineIndex: number): number {
		return this.getLineLength(outputLineIndex);
	}
}

export interface ILineBreaksComputerContext {
	getLineContent(lineNumber: number): string;
	getLineInjectedText(lineNumber: number): LineInjectedText[] | null;
}

export interface ILineBreaksComputerFactory {
	createLineBreaksComputer(
		context: ILineBreaksComputerContext,
		fontInfo: FontInfo,
		tabSize: number,
		wrappingColumn: number,
		wrappingIndent: WrappingIndent,
		wordBreak: 'normal' | 'keepAll',
		wrapOnEscapedLineFeeds: boolean,
	): ILineBreaksComputer;
}

export interface ILineBreaksComputer {
	addRequest(lineNumber: number, previousLineBreakData: ModelLineProjectionData | null): void;
	finalize(): (ModelLineProjectionData | null)[];
}
