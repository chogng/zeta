import { Disposable } from '../../../../base/common/lifecycle.js';
import { type ViewController } from '../../../browser/view/viewController.js';
import { type Selection } from '../../../common/core/selection.js';
import { type Range } from '../../../common/core/range.js';
import { USUAL_WORD_SEPARATORS } from '../../../common/core/wordHelper.js';
import { type CursorsController } from '../../../common/cursor/cursor.js';
import { TextDecorationCollection } from '../../../common/model/decorationCollection.js';

import type { ILanguageFeaturesService } from '../../../common/services/languageFeatures.js';
import { TrackedRangeStickiness } from '../../../common/model.js';

const MAX_SELECTION_HIGHLIGHTS = 10_000;

interface SelectionHighlighterOptions {
	readonly languageId: string;
	readonly languageFeaturesService: ILanguageFeaturesService;
	readonly enabled?: boolean;
	readonly multiline?: boolean;
	readonly maxLength?: number;
	readonly occurrenceHighlights?: boolean;
}

/** Owns textual matches for non-empty editor selections. */
export class SelectionHighlighter extends Disposable {
	private readonly enabled: boolean;
	private readonly multiline: boolean;
	private readonly maxLength: number;
	private readonly occurrenceHighlights: boolean;
	private readonly languageId: string;
	private readonly languageFeaturesService: ILanguageFeaturesService;
	private lastKey = '';

	constructor(
		view: ViewController,
		private readonly selections: CursorsController,
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
		this._register(selections.onDidChange(() => this.update()));
		this._register(selections.textModel.onDidChangeContent(() => this.update()));
		this.update();
	}

	private update(): void {
		const ranges = this.findRanges();
		const hasSemanticHighlights = this.occurrenceHighlights && this.languageFeaturesService.documentHighlightProvider.has(this.selections.textModel);
		const key = `${hasSemanticHighlights}:${ranges.map(range => `${this.selections.textModel.offsetAt(range.getStartPosition())}-${this.selections.textModel.offsetAt(range.getEndPosition())}`).join(',')}`;
		if (key === this.lastKey) return;
		this.lastKey = key;
		this.decorations.replaceAll(ranges.map(range => ({
			range,
			stickiness: TrackedRangeStickiness.NeverGrowsWhenTypingAtEdges,
			metadata: hasSemanticHighlights,
		})));
	}

	private findRanges(): readonly Range[] {
		if (!this.enabled) return Object.freeze([]);
		const selected = this.selections.selections;
		if (selected.some(selection => selection.isEmpty())) return Object.freeze([]);
		const source = selected[0]!;
		if (!this.multiline && source.getStartPosition().lineNumber !== source.getEndPosition().lineNumber) return Object.freeze([]);
		const text = this.selections.textModel.getTextInRange(source);
		if (!text || /^\s+$/u.test(text) || (this.maxLength > 0 && text.length > this.maxLength)) return Object.freeze([]);
		if (!selectionsContainSameText(this.selections, selected, text)) return Object.freeze([]);
		const word = this.selections.textModel.getWordAtPosition(source.getStartPosition());
		const wholeWord = word !== null && source.startLineNumber === source.endLineNumber && source.startColumn === word.startColumn && source.endColumn === word.endColumn;
		const matches = this.selections.textModel.findMatches(text, true, false, true, wholeWord ? USUAL_WORD_SEPARATORS : null, false, MAX_SELECTION_HIGHLIGHTS);
		return Object.freeze(matches.flatMap(match => {
			if (selected.some(selection => rangesIntersect(this.selections, match.range, selection))) return [];
			return [match.range];
		}));
	}
}

function validateSelectionHighlighter(view: ViewController, selections: CursorsController, decorations: TextDecorationCollection<boolean>, options: SelectionHighlighterOptions): void {
	if (view.viewport.textModel !== selections.textModel || selections.textModel !== decorations.textModel) throw new TypeError('Selection highlighter dependencies must share one text model');
	if (!options || typeof options !== 'object' || !options.languageId || !options.languageFeaturesService) throw new TypeError('Selection highlighter requires language services');
	if (options.enabled !== undefined && typeof options.enabled !== 'boolean') throw new TypeError('Selection highlighter enabled option must be boolean');
	if (options.multiline !== undefined && typeof options.multiline !== 'boolean') throw new TypeError('Selection highlighter multiline option must be boolean');
	if (options.occurrenceHighlights !== undefined && typeof options.occurrenceHighlights !== 'boolean') throw new TypeError('Selection highlighter semantic option must be boolean');
	if (options.maxLength !== undefined && (!Number.isSafeInteger(options.maxLength) || options.maxLength < 0)) throw new RangeError('Selection highlighter maximum length must be a non-negative integer');
}

function selectionsContainSameText(controller: CursorsController, selections: readonly Selection[], text: string): boolean {
	return selections.every(selection => controller.textModel.getTextInRange(selection) === text);
}

function rangesIntersect(controller: CursorsController, left: Range, right: Range): boolean {
	const leftStart = controller.textModel.offsetAt(left.getStartPosition());
	const leftEnd = controller.textModel.offsetAt(left.getEndPosition());
	const rightStart = controller.textModel.offsetAt(right.getStartPosition());
	const rightEnd = controller.textModel.offsetAt(right.getEndPosition());
	return leftStart < rightEnd && rightStart < leftEnd;
}
