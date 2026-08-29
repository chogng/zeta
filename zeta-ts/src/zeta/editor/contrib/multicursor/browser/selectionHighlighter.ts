import { Disposable } from '../../../../base/common/lifecycle.js';
import { type EditorView } from '../../../browser/view.js';
import { type TextSelection } from '../../../common/core/selection.js';
import { type TextRange } from '../../../common/core/text.js';
import { type EditorSelectionController } from '../../../common/cursor/cursor.js';
import { getWordSelectionRange } from '../../../common/cursor/cursorWordOperations.js';
import { TextDecorationCollection } from '../../../common/model/decorationCollection.js';
import { findTextMatches } from '../../../common/model/textModelSearch.js';
import { TrackedRangeStickiness } from '../../../common/model/trackedRange.js';
import type { ILanguageFeaturesService } from '../../../common/services/languageFeatures.js';

const MAX_SELECTION_HIGHLIGHTS = 10_000;

interface SelectionHighlighterOptions {
	readonly languageId: string;
	readonly languageFeaturesService: ILanguageFeaturesService;
	readonly enabled?: boolean;
	readonly multiline?: boolean;
	readonly maxLength?: number;
	readonly occurrenceHighlights?: boolean;
	readonly wordPattern?: () => RegExp | undefined;
}

/** Owns textual matches for non-empty editor selections. */
export class SelectionHighlighter extends Disposable {
	private readonly enabled: boolean;
	private readonly multiline: boolean;
	private readonly maxLength: number;
	private readonly occurrenceHighlights: boolean;
	private readonly languageId: string;
	private readonly languageFeaturesService: ILanguageFeaturesService;
	private readonly wordPattern: (() => RegExp | undefined) | undefined;
	private lastKey = '';

	constructor(
		view: EditorView,
		private readonly selections: EditorSelectionController,
		private readonly decorations: TextDecorationCollection<boolean>,
		options: SelectionHighlighterOptions,
	) {
		super();
		validateSelectionHighlighter(view, selections, decorations, options);
		this.enabled = options.enabled ?? true;
		this.multiline = options.multiline ?? false;
		this.maxLength = options.maxLength ?? 200;
		this.occurrenceHighlights = options.occurrenceHighlights ?? true;
		this.languageId = options.languageId;
		this.languageFeaturesService = options.languageFeaturesService;
		this.wordPattern = options.wordPattern;
		this._register(selections.onDidChange(() => this.update()));
		this._register(selections.textModel.onDidChange(() => this.update()));
		this.update();
	}

	private update(): void {
		const ranges = this.findRanges();
		const hasSemanticHighlights = this.occurrenceHighlights && this.languageFeaturesService.documentHighlightProvider.getProviders(this.languageId).length > 0;
		const key = `${hasSemanticHighlights}:${ranges.map(range => `${this.selections.textModel.offsetAt(range.start)}-${this.selections.textModel.offsetAt(range.end)}`).join(',')}`;
		if (key === this.lastKey) return;
		this.lastKey = key;
		this.decorations.replaceAll(ranges.map(range => ({
			range,
			stickiness: TrackedRangeStickiness.NeverGrowsAtEdges,
			metadata: hasSemanticHighlights,
		})));
	}

	private findRanges(): readonly TextRange[] {
		if (!this.enabled) return Object.freeze([]);
		const selected = this.selections.selections.selections;
		if (selected.some(selection => selection.collapsed)) return Object.freeze([]);
		const source = selected[this.selections.selections.primaryIndex]!;
		if (!this.multiline && source.range.start.lineIndex !== source.range.end.lineIndex) return Object.freeze([]);
		const text = this.selections.textModel.getTextInRange(source.range);
		if (!text || /^\s+$/u.test(text) || (this.maxLength > 0 && text.length > this.maxLength)) return Object.freeze([]);
		if (!selectionsContainSameText(this.selections, selected, text)) return Object.freeze([]);
		const wordPattern = this.wordPattern?.();
		const wordRange = getWordSelectionRange(this.selections.textModel, source.range.start, wordPattern);
		const wholeWord = rangesEqual(wordRange, source.range);
		const matches = findTextMatches(this.selections.textModel, {
			pattern: text,
			matchCase: true,
			wholeWord: wholeWord && !wordPattern,
		}, { resultLimit: MAX_SELECTION_HIGHLIGHTS });
		return Object.freeze(matches.flatMap(match => {
			if (selected.some(selection => rangesIntersect(this.selections, match.range, selection.range))) return [];
			if (wholeWord && wordPattern && !rangesEqual(getWordSelectionRange(this.selections.textModel, match.range.start, wordPattern), match.range)) return [];
			return [match.range];
		}));
	}
}

function validateSelectionHighlighter(view: EditorView, selections: EditorSelectionController, decorations: TextDecorationCollection<boolean>, options: SelectionHighlighterOptions): void {
	if (view.viewport.textModel !== selections.textModel || selections.textModel !== decorations.textModel) throw new TypeError('Selection highlighter dependencies must share one text model');
	if (!options || typeof options !== 'object' || !options.languageId || !options.languageFeaturesService) throw new TypeError('Selection highlighter requires language services');
	if (options.enabled !== undefined && typeof options.enabled !== 'boolean') throw new TypeError('Selection highlighter enabled option must be boolean');
	if (options.multiline !== undefined && typeof options.multiline !== 'boolean') throw new TypeError('Selection highlighter multiline option must be boolean');
	if (options.occurrenceHighlights !== undefined && typeof options.occurrenceHighlights !== 'boolean') throw new TypeError('Selection highlighter semantic option must be boolean');
	if (options.maxLength !== undefined && (!Number.isSafeInteger(options.maxLength) || options.maxLength < 0)) throw new RangeError('Selection highlighter maximum length must be a non-negative integer');
	if (options.wordPattern !== undefined && typeof options.wordPattern !== 'function') throw new TypeError('Selection highlighter word pattern resolver must be a function');
}

function selectionsContainSameText(controller: EditorSelectionController, selections: readonly TextSelection[], text: string): boolean {
	return selections.every(selection => controller.textModel.getTextInRange(selection.range) === text);
}

function rangesIntersect(controller: EditorSelectionController, left: TextRange, right: TextRange): boolean {
	const leftStart = controller.textModel.offsetAt(left.start);
	const leftEnd = controller.textModel.offsetAt(left.end);
	const rightStart = controller.textModel.offsetAt(right.start);
	const rightEnd = controller.textModel.offsetAt(right.end);
	return leftStart < rightEnd && rightStart < leftEnd;
}

function rangesEqual(left: TextRange, right: TextRange): boolean {
	return left.start.compareTo(right.start) === 0 && left.end.compareTo(right.end) === 0;
}
