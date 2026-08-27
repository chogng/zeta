import { Disposable } from '../../../../base/common/lifecycle.js';
import { type TextSelectionSet } from '../../../common/core/selection.js';
import { TextRange } from '../../../common/core/text.js';
import { getTextWordSegments } from '../../../common/core/textSegmentation.js';
import { type EditorSelectionController } from '../../../common/cursor/editorSelectionController.js';
import { getWordSelectionRange } from '../../../common/cursor/wordBoundary.js';
import { TextDecorationCollection } from '../../../common/model/decorationCollection.js';
import { type TextModel } from '../../../common/model/textModel.js';
import { findTextMatches } from '../../../common/model/textModelSearch.js';
import { TrackedRangeStickiness } from '../../../common/model/trackedRange.js';

const MAX_OCCURRENCE_HIGHLIGHTS = 10_000;

export interface OccurrenceHighlightControllerOptions {
	readonly wordPattern?: () => RegExp | undefined;
}

/** Owns current primary-word occurrence highlights for one editor. */
export class OccurrenceHighlightController extends Disposable {
	private lastKey = '';
	private readonly wordPattern: (() => RegExp | undefined) | undefined;

	constructor(
		private readonly selections: EditorSelectionController,
		private readonly decorations: TextDecorationCollection<void>,
		options: OccurrenceHighlightControllerOptions = {},
	) {
		super();
		try {
			if (selections.textModel !== decorations.textModel) {
				throw new TypeError('Stanza occurrence highlighting dependencies must share one text model');
			}
			if (options.wordPattern !== undefined && typeof options.wordPattern !== 'function') {
				throw new TypeError('Stanza occurrence highlight word pattern resolver must be a function');
			}
			this.wordPattern = options.wordPattern;
			this._register(selections.onDidChange(() => this.update()));
			this._register(selections.textModel.onDidChange(() => this.update()));
			this.update();
		} catch (error) {
			this.dispose();
			throw error;
		}
	}

	private update(): void {
		const model = this.selections.textModel;
		const ranges = getOccurrenceHighlightRanges(model, this.selections.selections, this.wordPattern?.());
		const key = `${model.version}:${ranges.map(range => `${model.offsetAt(range.start)}-${model.offsetAt(range.end)}`).join(',')}`;
		if (key === this.lastKey) return;
		this.lastKey = key;
		this.decorations.replaceAll(ranges.map(range => ({
			range,
			stickiness: TrackedRangeStickiness.NeverGrowsAtEdges,
			metadata: undefined,
		})));
	}
}

function getOccurrenceHighlightRanges(model: TextModel, selections: TextSelectionSet, wordPattern?: RegExp): readonly TextRange[] {
	const source = readOccurrenceSource(model, selections, wordPattern);
	if (!source) return Object.freeze([]);
	const matches = findTextMatches(model, {
		pattern: source.text,
		matchCase: true,
		wholeWord: source.wholeWord && !wordPattern,
	}, { resultLimit: MAX_OCCURRENCE_HIGHLIGHTS });
	return Object.freeze(matches.flatMap(match => wordPattern && source.wholeWord && !isPatternWord(model, match.range, wordPattern) ? [] : [match.range]));
}

function readOccurrenceSource(model: TextModel, selections: TextSelectionSet, wordPattern: RegExp | undefined): { readonly text: string; readonly wholeWord: boolean } | undefined {
	const selection = selections.primary;
	if (!selectionFitsModel(model, selection.range)) return undefined;
	if (!selection.collapsed) {
		if (selection.range.start.lineIndex !== selection.range.end.lineIndex) return undefined;
		const text = model.getTextInRange(selection.range);
		return text.length > 0 ? Object.freeze({ text, wholeWord: false }) : undefined;
	}
	const range = getWordSelectionRange(model, selection.active, wordPattern);
	if (range.empty) return undefined;
	const segment = wordPattern ? { wordLike: true } : getTextWordSegments(model.getLineContent(selection.active.lineIndex)).find(candidate =>
		candidate.start === range.start.columnIndex && candidate.end === range.end.columnIndex
	);
	if (!segment?.wordLike) return undefined;
	return Object.freeze({ text: model.getTextInRange(range), wholeWord: true });
}

function selectionFitsModel(model: TextModel, range: TextRange): boolean {
	return positionFitsModel(model, range.start.lineIndex, range.start.columnIndex) &&
		positionFitsModel(model, range.end.lineIndex, range.end.columnIndex);
}

function positionFitsModel(model: TextModel, lineIndex: number, columnIndex: number): boolean {
	return Number.isSafeInteger(lineIndex) &&
		Number.isSafeInteger(columnIndex) &&
		lineIndex >= 0 &&
		columnIndex >= 0 &&
		lineIndex < model.lineCount &&
		columnIndex <= model.getLineLength(lineIndex);
}

function isPatternWord(model: TextModel, range: TextRange, wordPattern: RegExp): boolean {
	const selected = getWordSelectionRange(model, range.start, wordPattern);
	return selected.start.compareTo(range.start) === 0 && selected.end.compareTo(range.end) === 0;
}
