import { Disposable } from "../../../../base/common/lifecycle.js";
import { TextDecorationCollection } from "../../../common/model/decorationCollection.js";
import { type EditorSelectionController } from "../../../common/cursor/editorSelectionController.js";
import { type LanguageBracketPairs } from "../../../common/languages/languageBracketPairs.js";
import { type TextRange } from "../../../common/core/text.js";
import { TrackedRangeStickiness } from "../../../common/model/trackedRange.js";

/** Projects current collapsed-cursor bracket matches into caller-owned decorations. */
export class BracketMatchController extends Disposable {
	constructor(
		private readonly selections: EditorSelectionController,
		private readonly bracketPairs: LanguageBracketPairs,
		private readonly decorations: TextDecorationCollection<void>,
		private readonly mode: "never" | "near" | "always",
	) {
		super();
		try {
			if (selections.textModel !== bracketPairs.textModel || selections.textModel !== decorations.textModel) {
				throw new TypeError("Stanza bracket matching dependencies must share one text model");
			}
			this._register(selections.onDidChange(() => this.update()));
			this._register(bracketPairs.onDidChange(() => this.update()));
			this.update();
		} catch (error) {
			this.dispose();
			throw error;
		}
	}

	private update(): void {
		if (this.mode === "never") {
			this.decorations.replaceAll([]);
			return;
		}
		const ranges = new Map<string, TextRange>();
		for (const selection of this.selections.selections.selections) {
			if (!selection.collapsed) continue;
			const match = this.bracketPairs.matchBracket(selection.active)
				?? (this.mode === "always" ? this.bracketPairs.findEnclosingBrackets(selection.active) : undefined);
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
