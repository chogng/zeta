import "./media/unicodeHighlighter.css";
import { Disposable } from "../../../../base/common/lifecycle.js";
import { type TextDecorationCollection } from "../../../common/model/decorationCollection.js";
import { TrackedRangeStickiness } from "../../../common/model/trackedRange.js";
import { type TextModel } from "../../../common/model/textModel.js";
import { type IEditorWorkerClient } from "../../../common/services/editorWorker.js";
import { type UnicodeHighlight } from "../common/unicodeHighlighter.js";

/** Maintains Unicode warning ranges as a feature-owned decoration collection. */
export class UnicodeHighlighterController extends Disposable {
	private lastVersion = -1;

	constructor(private readonly model: TextModel, readonly decorations: TextDecorationCollection<UnicodeHighlight>, private readonly editorWorker: IEditorWorkerClient, private readonly onError: (error: unknown) => void) {
		super();
		if (decorations.textModel !== model) throw new TypeError("Stanza Unicode highlighter dependencies must share a text model");
		this._register(model.onDidChange(() => this.update()));
		this.update();
	}

	private update(): void {
		if (this.lastVersion === this.model.version) return;
		const version = this.lastVersion = this.model.version;
		void this.compute(version);
	}

	private async compute(version: number): Promise<void> {
		try {
			const highlights = await this.editorWorker.computeUnicodeHighlights();
			if (!highlights || this.isDisposed || this.model.version !== version) return;
			this.decorations.replaceAll(highlights.map(highlight => ({ range: highlight.range, stickiness: TrackedRangeStickiness.NeverGrowsAtEdges, metadata: highlight })));
		} catch (error) {
			if (!this.isDisposed && this.model.version === version) {
				this.lastVersion = -1;
				this.onError(error);
			}
		}
	}
}
