import { AccessibilitySupport } from '../../../../platform/accessibility/common/accessibility.js';
import { type IKeybindingService } from '../../../../platform/keybinding/common/keybinding.js';
import * as nls from '../../../../nls.js';
import { type IComputedEditorOptions, EditorOption } from '../../../common/config/editorOptions.js';
import { Position } from '../../../common/core/position.js';
import { Range } from '../../../common/core/range.js';
import { Selection, SelectionDirection } from '../../../common/core/selection.js';
import { EndOfLinePreference } from '../../../common/model.js';
import { type ISimpleModel } from '../../../common/viewModel/screenReaderSimpleModel.js';
import { type TextModel } from '../../../common/model/textModel.js';

export interface ScreenReaderSegment {
	readonly modelStartOffset: number;
	readonly modelEndOffset: number;
	readonly contentStartOffset: number;
	readonly contentEndOffset: number;
}

export interface MappedScreenReaderContentState extends ISimpleScreenReaderContentState {
	readonly startOffset: number;
	readonly endOffset: number;
	readonly segments: readonly ScreenReaderSegment[];
}

/** Creates a bounded text projection while retaining model/content offset mappings. */
export class MappedScreenReaderStrategy {
	fromEditorSelection(model: TextModel, selection: Selection, linesPerPage: number, trimLongText: boolean): MappedScreenReaderContentState {
		const pageSize = Math.max(1, Math.floor(linesPerPage));
		const firstLine = Math.max(1, selection.startLineNumber - pageSize);
		const lastLine = Math.min(model.lineCount, selection.endLineNumber + pageSize);
		let startOffset = model.offsetAt(new Position(firstLine, 1));
		let endOffset = lastLine === model.lineCount ? model.length : model.offsetAt(new Position(lastLine + 1, 1));
		const selectionStart = model.offsetAt(selection.getStartPosition());
		const selectionEnd = model.offsetAt(selection.getEndPosition());
		if (trimLongText) {
			startOffset = Math.max(startOffset, selectionStart - 500);
			endOffset = Math.min(endOffset, selectionEnd + 500);
		}
		const value = model.createVersionedSnapshot().getTextBetweenOffsets(startOffset, endOffset);
		const segment: ScreenReaderSegment = {
			modelStartOffset: startOffset,
			modelEndOffset: endOffset,
			contentStartOffset: 0,
			contentEndOffset: value.length,
		};
		const start = selectionStart - startOffset;
		const end = selectionEnd - startOffset;
		const rtl = selection.getDirection() === SelectionDirection.RTL;
		return {
			value,
			selection,
			selectionStart: rtl ? end : start,
			selectionEnd: rtl ? start : end,
			startPositionWithinEditor: model.positionAt(startOffset),
			newlineCountBeforeSelection: newlinecount(value.slice(0, start)),
			startOffset,
			endOffset,
			segments: [segment],
		};
	}
}

export function modelOffsetAtContentOffset(
	state: Pick<MappedScreenReaderContentState, 'value' | 'segments'>,
	offset: number,
	affinity: 'start' | 'end' = 'start',
): number {
	if (state.segments.length === 0) return 0;
	const valueOffset = Math.max(0, Math.min(state.value.length, offset));
	for (let index = 0; index < state.segments.length; index += 1) {
		const segment = state.segments[index]!;
		if (valueOffset < segment.contentStartOffset) {
			return affinity === 'end' && index > 0
				? state.segments[index - 1]!.modelEndOffset
				: segment.modelStartOffset;
		}
		if (valueOffset < segment.contentEndOffset || valueOffset === segment.contentEndOffset && affinity === 'end') {
			return segment.modelStartOffset + Math.max(0, valueOffset - segment.contentStartOffset);
		}
	}
	return state.segments.at(-1)!.modelEndOffset;
}

export interface IPagedScreenReaderStrategy<T> {
	fromEditorSelection(model: ISimpleModel, selection: Selection, linesPerPage: number, trimLongText: boolean): T;
}

export interface ISimpleScreenReaderContentState {
	value: string;
	selectionStart: number;
	selectionEnd: number;
	selection: Selection;
	startPositionWithinEditor: Position;
	newlineCountBeforeSelection: number;
}

export class SimplePagedScreenReaderStrategy implements IPagedScreenReaderStrategy<ISimpleScreenReaderContentState> {
	private _getPageOfLine(lineNumber: number, linesPerPage: number): number {
		return Math.floor((lineNumber - 1) / linesPerPage);
	}

	private _getRangeForPage(page: number, linesPerPage: number): Range {
		const offset = page * linesPerPage;
		return new Range(offset + 1, 1, offset + linesPerPage + 1, 1);
	}

	public fromEditorSelection(model: ISimpleModel, selection: Selection, linesPerPage: number, trimLongText: boolean): ISimpleScreenReaderContentState {
		const limitCharacters = 500;
		const selectionStartPage = this._getPageOfLine(selection.startLineNumber, linesPerPage);
		const selectionStartPageRange = this._getRangeForPage(selectionStartPage, linesPerPage);
		const selectionEndPage = this._getPageOfLine(selection.endLineNumber, linesPerPage);
		const selectionEndPageRange = this._getRangeForPage(selectionEndPage, linesPerPage);

		let pretextRange = selectionStartPageRange.intersectRanges(new Range(1, 1, selection.startLineNumber, selection.startColumn))!;
		if (trimLongText && model.getValueLengthInRange(pretextRange, EndOfLinePreference.LF) > limitCharacters) {
			pretextRange = Range.fromPositions(model.modifyPosition(pretextRange.getEndPosition(), -limitCharacters), pretextRange.getEndPosition());
		}
		const pretext = model.getValueInRange(pretextRange, EndOfLinePreference.LF);

		const lastLine = model.getLineCount();
		let posttextRange = selectionEndPageRange.intersectRanges(new Range(selection.endLineNumber, selection.endColumn, lastLine, model.getLineMaxColumn(lastLine)))!;
		if (trimLongText && model.getValueLengthInRange(posttextRange, EndOfLinePreference.LF) > limitCharacters) {
			posttextRange = Range.fromPositions(posttextRange.getStartPosition(), model.modifyPosition(posttextRange.getStartPosition(), limitCharacters));
		}
		const posttext = model.getValueInRange(posttextRange, EndOfLinePreference.LF);

		let text: string;
		if (selectionStartPage === selectionEndPage || selectionStartPage + 1 === selectionEndPage) {
			text = model.getValueInRange(selection, EndOfLinePreference.LF);
		} else {
			text = model.getValueInRange(selectionStartPageRange.intersectRanges(selection)!, EndOfLinePreference.LF)
				+ String.fromCharCode(8230)
				+ model.getValueInRange(selectionEndPageRange.intersectRanges(selection)!, EndOfLinePreference.LF);
		}
		if (trimLongText && text.length > 2 * limitCharacters) {
			text = text.substring(0, limitCharacters) + String.fromCharCode(8230) + text.substring(text.length - limitCharacters);
		}

		const leftToRight = selection.getDirection() === SelectionDirection.LTR;
		return {
			value: pretext + text + posttext,
			selection,
			selectionStart: leftToRight ? pretext.length : pretext.length + text.length,
			selectionEnd: leftToRight ? pretext.length + text.length : pretext.length,
			startPositionWithinEditor: pretextRange.getStartPosition(),
			newlineCountBeforeSelection: pretextRange.endLineNumber - pretextRange.startLineNumber,
		};
	}
}

export function ariaLabelForScreenReaderContent(options: IComputedEditorOptions, keybindingService: IKeybindingService) {
	if (options.get(EditorOption.accessibilitySupport) === AccessibilitySupport.Disabled) {
		const hasToggleKeybinding = keybindingService.lookupKeybinding('editor.action.toggleScreenReaderAccessibilityMode') !== undefined;
		return nls.localize('editor', 'accessibilityModeOff', hasToggleKeybinding
			? 'The editor is not accessible at this time. Use the configured accessibility-mode shortcut to enable it.'
			: 'The editor is not accessible at this time. Enable screen reader optimized mode from the command menu.');
	}
	return options.get(EditorOption.ariaLabel);
}

export function newlinecount(text: string): number {
	let result = 0;
	for (const character of text) if (character === '\n') result += 1;
	return result;
}
