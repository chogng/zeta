/*---------------------------------------------------------------------------------------------
 *  Copyright (c) Microsoft Corporation. All rights reserved.
 *  Licensed under the MIT License. See License.txt in the project root for license information.
 *--------------------------------------------------------------------------------------------*/

import { Range } from '../core/range.js';
import { Selection } from '../core/selection.js';
import { IPartialViewLinesViewportData, IViewModel, IViewWhitespaceViewportData, ViewLineRenderingData } from '../viewModel.js';
import { ViewModelDecoration } from '../viewModel/viewModelDecoration.js';
import { type EditorLineRange } from '../viewModel/editorViewportContracts.js';

export interface EditorViewportDataOptions {
	readonly modelVersion: number;
	readonly lineHeight: number;
	readonly visibleLines: EditorLineRange;
	readonly renderLines: EditorLineRange;
	readonly renderTop: number;
	readonly relativeVerticalOffset?: readonly number[];
}

/** Immutable render snapshot for the visible and overscan line windows. */
export class EditorViewportData {
	readonly startLineIndex: number;
	readonly endLineIndexExclusive: number;
	readonly relativeVerticalOffset: readonly number[];

	constructor(readonly options: EditorViewportDataOptions) {
		if (!Number.isSafeInteger(options.modelVersion) || options.modelVersion < 0) throw new RangeError('Viewport model version is invalid');
		if (!Number.isFinite(options.lineHeight) || options.lineHeight <= 0) throw new RangeError('Viewport line height is invalid');
		this.startLineIndex = options.renderLines.startLineIndex;
		this.endLineIndexExclusive = options.renderLines.endLineIndexExclusive;
		const count = this.endLineIndexExclusive - this.startLineIndex;
		this.relativeVerticalOffset = options.relativeVerticalOffset ?? Array.from({ length: count }, (_, index) => options.renderTop + index * options.lineHeight);
		if (this.relativeVerticalOffset.length !== count) throw new RangeError('Viewport offsets do not cover the render window');
	}

	get modelVersion(): number { return this.options.modelVersion; }
	get lineHeight(): number { return this.options.lineHeight; }
	get visibleLines(): EditorLineRange { return this.options.visibleLines; }
	get renderLines(): EditorLineRange { return this.options.renderLines; }
	get renderTop(): number { return this.options.renderTop; }

	getLineTop(lineIndex: number): number {
		if (lineIndex < this.startLineIndex || lineIndex >= this.endLineIndexExclusive) throw new RangeError('Line is outside the render window');
		return this.relativeVerticalOffset[lineIndex - this.startLineIndex]!;
	}
}

/**
 * Contains all data needed to render at a specific viewport.
 */
export class ViewportData {

	public readonly selections: Selection[];

	/**
	 * The line number at which to start rendering (inclusive).
	 */
	public readonly startLineNumber: number;

	/**
	 * The line number at which to end rendering (inclusive).
	 */
	public readonly endLineNumber: number;

	/**
	 * relativeVerticalOffset[i] is the `top` position for line at `i` + `startLineNumber`.
	 */
	public readonly relativeVerticalOffset: number[];

	/**
	 * The viewport as a range (startLineNumber,1) -> (endLineNumber,maxColumn(endLineNumber)).
	 */
	public readonly visibleRange: Range;

	/**
	 * Value to be substracted from `scrollTop` (in order to vertical offset numbers < 1MM)
	 */
	public readonly bigNumbersDelta: number;

	/**
	 * Positioning information about gaps whitespace.
	 */
	public readonly whitespaceViewportData: IViewWhitespaceViewportData[];

	private readonly _model: IViewModel;

	public readonly lineHeight: number;

	constructor(
		selections: Selection[],
		partialData: IPartialViewLinesViewportData,
		whitespaceViewportData: IViewWhitespaceViewportData[],
		model: IViewModel
	) {
		this.selections = selections;
		this.startLineNumber = partialData.startLineNumber | 0;
		this.endLineNumber = partialData.endLineNumber | 0;
		this.relativeVerticalOffset = partialData.relativeVerticalOffset;
		this.bigNumbersDelta = partialData.bigNumbersDelta | 0;
		this.lineHeight = partialData.lineHeight | 0;
		this.whitespaceViewportData = whitespaceViewportData;

		this._model = model;

		this.visibleRange = new Range(
			partialData.startLineNumber,
			this._model.getLineMinColumn(partialData.startLineNumber),
			partialData.endLineNumber,
			this._model.getLineMaxColumn(partialData.endLineNumber)
		);
	}

	public getViewLineRenderingData(lineNumber: number): ViewLineRenderingData {
		return this._model.getViewportViewLineRenderingData(this.visibleRange, lineNumber);
	}

	public getDecorationsInViewport(): ViewModelDecoration[] {
		return this._model.getDecorationsInViewport(this.visibleRange);
	}
}
