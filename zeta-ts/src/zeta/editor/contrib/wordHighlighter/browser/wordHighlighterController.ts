import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import { TextDecorationCollection } from "../../../common/model/decorationCollection.js";
import { type EditorSelectionController } from "../../../common/cursor/editorSelectionController.js";
import { getOccurrenceHighlightRanges } from "../common/wordHighlighter.js";
import { TrackedRangeStickiness } from "../../../common/model/trackedRange.js";

export interface OccurrenceHighlightControllerOptions {
	readonly wordPattern?: () => RegExp | undefined;
}

/** Projects current primary-word occurrences through a caller-owned decoration collection. */
export class OccurrenceHighlightController extends DisposableOwner {
	private lastKey = "";

	constructor(
		private readonly selections: EditorSelectionController,
		private readonly decorations: TextDecorationCollection<void>,
		options: OccurrenceHighlightControllerOptions = {},
	) {
		super();
		try {
			if (selections.textModel !== decorations.textModel) {
				throw new TypeError("Stanza occurrence highlighting dependencies must share one text model");
			}
			if (options.wordPattern !== undefined && typeof options.wordPattern !== "function") {
				throw new TypeError("Stanza occurrence highlight word pattern resolver must be a function");
			}
			this.wordPattern = options.wordPattern;
			this.own(selections.onDidChange(() => this.update()));
			this.own(selections.textModel.onDidChange(() => this.update()));
			this.update();
		} catch (error) {
			this.dispose();
			throw error;
		}
	}

	private readonly wordPattern: (() => RegExp | undefined) | undefined;

	private update(): void {
		const model = this.selections.textModel;
		const ranges = getOccurrenceHighlightRanges(model, this.selections.selections, this.wordPattern?.());
		const key = `${model.version}:${ranges.map(range => `${model.offsetAt(range.start)}-${model.offsetAt(range.end)}`).join(",")}`;
		if (key === this.lastKey) return;
		this.lastKey = key;
		this.decorations.replaceAll(ranges.map(range => ({
			range,
			stickiness: TrackedRangeStickiness.NeverGrowsAtEdges,
			metadata: undefined,
		})));
	}
}
