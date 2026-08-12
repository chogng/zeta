import "./media/unicodeHighlighter.css";
import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import { type TextDecorationCollection } from "../../../common/model/decorationCollection.js";
import { TrackedRangeStickiness } from "../../../common/model/trackedRange.js";
import { type TextModel } from "../../../common/model/textModel.js";
import { findUnicodeHighlights, type UnicodeHighlight } from "../common/unicodeHighlighter.js";

/** Maintains Unicode warning ranges as a feature-owned decoration collection. */
export class UnicodeHighlighterController extends DisposableOwner {
  private lastVersion = -1;

  constructor(private readonly model: TextModel, readonly decorations: TextDecorationCollection<UnicodeHighlight>) {
    super();
    if (decorations.textModel !== model) throw new TypeError("Alpha Unicode highlighter dependencies must share a text model");
    this.own(model.onDidChange(() => this.update()));
    this.update();
  }

  private update(): void {
    if (this.lastVersion === this.model.version) return;
    this.lastVersion = this.model.version;
    this.decorations.replaceAll(findUnicodeHighlights(this.model).map(highlight => ({ range: highlight.range, stickiness: TrackedRangeStickiness.NeverGrowsAtEdges, metadata: highlight })));
  }
}
