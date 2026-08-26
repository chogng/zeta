import { TextSelection } from '../../../common/core/selection.js';
import { TextPosition } from '../../../common/core/text.js';
import { type TextModel } from '../../../common/model/textModel.js';

const SCREEN_READER_PAGE_SEPARATOR = String.fromCharCode(8230);
const SCREEN_READER_TRIM_LENGTH = 500;

/** A model range and its corresponding UTF-16 range in the projected value. */
export interface SimpleScreenReaderContentSegment {
	readonly modelStartOffset: number;
	readonly modelEndOffset: number;
	readonly contentStartOffset: number;
	readonly contentEndOffset: number;
}

/** The common screen-reader projection contract used by textarea input. */
export interface ISimpleScreenReaderContentState {
	readonly value: string;
	/** The direction-aware selection offsets used by TextAreaState. */
	readonly selectionStart: number;
	readonly selectionEnd: number;
	readonly selection: TextSelection;
	readonly startPositionWithinEditor: TextPosition;
	readonly newlineCountBeforeSelection: number;
	readonly startOffset: number;
	readonly endOffset: number;
	readonly segments: readonly SimpleScreenReaderContentSegment[];
}

export interface IPagedScreenReaderStrategy<T> {
	fromEditorSelection(model: TextModel, selection: TextSelection, linesPerPage: number, trimLongText: boolean): T;
}

/**
 * Creates the line-oriented projection used by the textarea accessibility
 * path. The selection's surrounding page remains readable while very large
 * ranges are bounded to keep browser text controls responsive.
 */
export class SimplePagedScreenReaderStrategy implements IPagedScreenReaderStrategy<ISimpleScreenReaderContentState> {
	fromEditorSelection(model: TextModel, selection: TextSelection, linesPerPage: number, trimLongText: boolean): ISimpleScreenReaderContentState {
		const pageSize = normalizePageSize(linesPerPage);
		const snapshot = model.createSnapshot();
		const selectionStart = model.offsetAt(selection.range.start);
		const selectionEnd = model.offsetAt(selection.range.end);
		const startPage = Math.floor(selection.range.start.lineIndex / pageSize);
		const endPage = Math.floor(selection.range.end.lineIndex / pageSize);
		const startPageRange = pageRange(model, startPage, pageSize);
		const endPageRange = pageRange(model, endPage, pageSize);
		const selectionRanges = startPage === endPage || startPage + 1 === endPage
			? [{ startOffset: selectionStart, endOffset: selectionEnd }]
			: [
				{ startOffset: selectionStart, endOffset: startPageRange.endOffset },
				{ startOffset: endPageRange.startOffset, endOffset: selectionEnd },
			];

		let value = '';
		const segments: SimpleScreenReaderContentSegment[] = [];
		const appendRange = (startOffset: number, endOffset: number): void => {
			const safeStartOffset = Math.min(startOffset, endOffset);
			const safeEndOffset = Math.max(startOffset, endOffset);
			const contentStartOffset = value.length;
			value += snapshot.getTextBetweenOffsets(safeStartOffset, safeEndOffset);
			segments.push(Object.freeze({
				modelStartOffset: safeStartOffset,
				modelEndOffset: safeEndOffset,
				contentStartOffset,
				contentEndOffset: value.length,
			}));
		};
		const appendSeparator = (): void => {
			value += SCREEN_READER_PAGE_SEPARATOR;
		};

		appendBoundedRange(
			startPageRange.startOffset,
			selectionStart,
			trimLongText,
			'last',
			appendRange,
		);
		appendSelectionRanges(selectionRanges, trimLongText, appendRange, appendSeparator);
		appendBoundedRange(
			selectionEnd,
			endPageRange.endOffset,
			trimLongText,
			'first',
			appendRange,
		);

		const firstSegment = segments[0] ?? Object.freeze({
			modelStartOffset: selectionStart,
			modelEndOffset: selectionEnd,
			contentStartOffset: 0,
			contentEndOffset: 0,
		});
		const lastSegment = segments.at(-1) ?? firstSegment;
		const mappingState: Pick<ISimpleScreenReaderContentState, 'value' | 'segments' | 'startOffset' | 'endOffset'> = {
			value,
			segments,
			startOffset: firstSegment.modelStartOffset,
			endOffset: lastSegment.modelEndOffset,
		};
		const orderedSelectionStart = contentOffsetAtModelOffset(mappingState, selectionStart, 'start');
		const orderedSelectionEnd = contentOffsetAtModelOffset(mappingState, selectionEnd, 'end');
		const directionAwareSelectionStart = selection.direction === 'backward'
			? orderedSelectionEnd
			: orderedSelectionStart;
		const directionAwareSelectionEnd = selection.direction === 'backward'
			? orderedSelectionStart
			: orderedSelectionEnd;
		const startPosition = model.positionAt(firstSegment.modelStartOffset);
		return Object.freeze({
			value,
			selectionStart: directionAwareSelectionStart,
			selectionEnd: directionAwareSelectionEnd,
			selection,
			startPositionWithinEditor: startPosition,
			newlineCountBeforeSelection: newlineCount(value.slice(0, orderedSelectionStart)),
			startOffset: firstSegment.modelStartOffset,
			endOffset: lastSegment.modelEndOffset,
			segments: Object.freeze(segments),
		});
	}
}

/** Maps a model offset to a projected UTF-16 offset, including omitted pages. */
export function contentOffsetAtModelOffset(
	state: Pick<ISimpleScreenReaderContentState, 'segments' | 'startOffset' | 'endOffset'>,
	modelOffset: number,
	affinity: 'start' | 'end' = 'start',
): number {
	if (state.segments.length === 0) return 0;
	const offset = clampModelOffset(modelOffset, state.startOffset, state.endOffset);
	let previous: SimpleScreenReaderContentSegment | undefined;
	for (const segment of state.segments) {
		if (offset < segment.modelStartOffset) {
			return affinity === 'end'
				? segment.contentStartOffset
				: previous?.contentEndOffset ?? segment.contentStartOffset;
		}
		if (offset <= segment.modelEndOffset) {
			return segment.contentStartOffset + offset - segment.modelStartOffset;
		}
		previous = segment;
	}
	return state.segments.at(-1)!.contentEndOffset;
}

/** Maps a projected UTF-16 offset back to the nearest model offset. */
export function modelOffsetAtContentOffset(
	state: Pick<ISimpleScreenReaderContentState, 'value' | 'segments'>,
	contentOffset: number,
	affinity: 'start' | 'end' = 'start',
): number {
	if (state.segments.length === 0) return 0;
	const offset = clampContentOffset(contentOffset, state.value.length);
	let previous: SimpleScreenReaderContentSegment | undefined;
	for (const segment of state.segments) {
		if (previous && offset >= previous.contentEndOffset && offset <= segment.contentStartOffset) {
			return affinity === 'end' ? segment.modelStartOffset : previous.modelEndOffset;
		}
		if (offset >= segment.contentStartOffset && offset <= segment.contentEndOffset) {
			return segment.modelStartOffset + offset - segment.contentStartOffset;
		}
		previous = segment;
	}
	return state.segments.at(-1)!.modelEndOffset;
}

export function newlineCount(value: string): number {
	let result = 0;
	for (const character of value) {
		if (character === '\n') result += 1;
	}
	return result;
}

function pageRange(model: TextModel, page: number, linesPerPage: number): { readonly startOffset: number; readonly endOffset: number } {
	const startLineIndex = page * linesPerPage;
	const endLineIndex = Math.min(model.lineCount, startLineIndex + linesPerPage);
	const startOffset = model.offsetAt(TextPosition.at(startLineIndex, 0));
	const endOffset = endLineIndex === model.lineCount
		? model.length
		: model.offsetAt(TextPosition.at(endLineIndex, 0));
	return { startOffset, endOffset };
}

function appendBoundedRange(
	startOffset: number,
	endOffset: number,
	trimLongText: boolean,
	direction: 'first' | 'last',
	appendRange: (startOffset: number, endOffset: number) => void,
): void {
	if (!trimLongText || endOffset - startOffset <= SCREEN_READER_TRIM_LENGTH) {
		appendRange(startOffset, endOffset);
		return;
	}
	if (direction === 'first') {
		appendRange(startOffset, startOffset + SCREEN_READER_TRIM_LENGTH);
		return;
	}
	appendRange(endOffset - SCREEN_READER_TRIM_LENGTH, endOffset);
}

function appendSelectionRanges(
	ranges: readonly { readonly startOffset: number; readonly endOffset: number }[],
	trimLongText: boolean,
	appendRange: (startOffset: number, endOffset: number) => void,
	appendSeparator: () => void,
): void {
	const totalLength = ranges.reduce((total, range) => total + range.endOffset - range.startOffset, 0);
	if (!trimLongText || totalLength <= SCREEN_READER_TRIM_LENGTH * 2) {
		for (const [index, range] of ranges.entries()) {
			if (index > 0) appendSeparator();
			appendRange(range.startOffset, range.endOffset);
		}
		return;
	}
	const first = ranges[0]!;
	const last = ranges.at(-1)!;
	appendRange(first.startOffset, Math.min(first.endOffset, first.startOffset + SCREEN_READER_TRIM_LENGTH));
	appendSeparator();
	appendRange(Math.max(last.startOffset, last.endOffset - SCREEN_READER_TRIM_LENGTH), last.endOffset);
}

function normalizePageSize(value: number): number {
	if (!Number.isSafeInteger(value) || value < 1) throw new RangeError('Screen-reader page size must be a positive safe integer');
	return value;
}

function clampModelOffset(offset: number, startOffset: number, endOffset: number): number {
	return Math.min(Math.max(Number.isSafeInteger(offset) ? offset : startOffset, startOffset), endOffset);
}

function clampContentOffset(offset: number, length: number): number {
	return Math.min(Math.max(Number.isSafeInteger(offset) ? offset : 0, 0), length);
}
