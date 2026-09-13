import { Disposable } from "../../../../base/common/lifecycle.js";
import { TextDecorationCollection } from "../../../common/model/decorationCollection.js";
import { type LanguageBracketPairs } from "../../../common/languages/languageBracketPairs.js";
import { type Range } from "../../../common/core/range.js";
import { TrackedRangeStickiness } from '../../../common/model.js';
import { type ICodeEditor } from '../../../browser/editorBrowser.js';


/** Projects current collapsed-cursor bracket matches into caller-owned decorations. */
export class BracketMatchController extends Disposable {
	constructor(
		private readonly editor: ICodeEditor,
		private readonly bracketPairs: LanguageBracketPairs,
		private readonly decorations: TextDecorationCollection<void>,
		private readonly mode: "never" | "near" | "always",
	) {
		super();
		try {
			if (editor.getModel() !== bracketPairs.textModel || editor.getModel() !== decorations.textModel) {
				throw new TypeError("Stanza bracket matching dependencies must share one text model");
			}
			this._register(editor.onDidChangeCursorSelection(() => this.update()));
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
		const ranges = new Map<string, Range>();
		for (const selection of this.editor.getSelections()!) {
			if (!selection.isEmpty()) continue;
			const match = this.bracketPairs.matchBracket(selection.getPosition())
				?? (this.mode === "always" ? this.bracketPairs.findEnclosingBrackets(selection.getPosition()) : undefined);
			if (!match) continue;
			ranges.set(rangeKey(match.opening), match.opening);
			ranges.set(rangeKey(match.closing), match.closing);
		}
		this.decorations.replaceAll([...ranges.values()].map(range => ({
			range,
			stickiness: TrackedRangeStickiness.NeverGrowsWhenTypingAtEdges,
			options: { description: 'bracket-match', className: 'bracket-match' },
			metadata: undefined,
		})));
	}
}

function rangeKey(range: Range): string {
	return `${range.startLineNumber}:${range.startColumn}-${range.endLineNumber}:${range.endColumn}`;
}
