import { Disposable } from "../../../../base/common/lifecycle.js";
import { TextDecorationCollection } from "../../../common/model/decorationCollection.js";
import { type EditorSelectionController } from "../../../common/cursor/editorSelectionController.js";
import { type LanguageBracketMatcher } from "../common/bracketMatching.js";
import { type TextRange } from "../../../common/core/text.js";
import { TrackedRangeStickiness } from "../../../common/model/trackedRange.js";

/** Projects current collapsed-cursor bracket matches into caller-owned decorations. */
export class BracketMatchController extends Disposable {
	constructor(
		private readonly selections: EditorSelectionController,
		private readonly matcher: LanguageBracketMatcher,
		private readonly decorations: TextDecorationCollection<void>,
	) {
		super();
		try {
			if (selections.textModel !== matcher.textModel || selections.textModel !== decorations.textModel) {
				throw new TypeError("Stanza bracket matching dependencies must share one text model");
			}
			this._register(selections.onDidChange(() => this.update()));
			this._register(matcher.textModel.onDidChange(() => this.update()));
			this.update();
		} catch (error) {
			this.dispose();
			throw error;
		}
	}

	private update(): void {
		const ranges = new Map<string, TextRange>();
		for (const selection of this.selections.selections.selections) {
			if (!selection.collapsed) continue;
			const match = this.matcher.findMatch(selection.active);
			if (!match) continue;
			ranges.set(rangeKey(match.opening), match.opening);
			ranges.set(rangeKey(match.closing), match.closing);
		}
		this.decorations.replaceAll([...ranges.values()].map(range => ({
			range,
			stickiness: TrackedRangeStickiness.NeverGrowsAtEdges,
			metadata: undefined,
		})));
	}
}

function rangeKey(range: { readonly start: { readonly lineIndex: number; readonly columnIndex: number }; readonly end: { readonly lineIndex: number; readonly columnIndex: number } }): string {
	return `${range.start.lineIndex}:${range.start.columnIndex}-${range.end.lineIndex}:${range.end.columnIndex}`;
}
