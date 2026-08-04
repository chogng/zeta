import "./media/unusualLineTerminators.css";
import { DisposableOwner } from "../../../../../base/common/lifecycle.js";
import { type TextDecorationCollection } from "../../../common/model/decorationCollection.js";
import { TrackedRangeStickiness } from "../../../common/model/trackedRange.js";
import { type TextModel } from "../../../common/model/textModel.js";
import { findUnusualLineTerminators } from "../common/unusualLineTerminators.js";

/** Highlights unusual Unicode line separators without changing the canonical LF model. */
export class AlphaUnusualLineTerminatorsController extends DisposableOwner {
  private lastVersion = -1;

  constructor(private readonly model: TextModel, readonly decorations: TextDecorationCollection<void>) {
    super();
    if (decorations.textModel !== model) throw new TypeError("Alpha unusual line terminator dependencies must share a text model");
    this.own(model.onDidChange(() => this.update()));
    this.update();
  }

  private update(): void {
    if (this.lastVersion === this.model.version) return;
    this.lastVersion = this.model.version;
    this.decorations.replaceAll(findUnusualLineTerminators(this.model).map(range => ({ range, stickiness: TrackedRangeStickiness.NeverGrowsAtEdges, metadata: undefined })));
  }
}
