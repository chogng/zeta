import "./composition.css";
import { type TextRange } from "../../../common/core/text.js";
import { type TextModel } from "../../../common/model/textModel.js";
import { TrackedRangeStickiness, type TrackedRange } from "../../../common/model/trackedRange.js";
import { type EditorViewportLayout } from "../../../common/viewLayout/editorViewportModel.js";
import { projectAsterCompositionOverlay } from "./compositionProjection.js";
import { EditorOverlayPart, EditorViewContext } from "../viewPart.js";

/** Owns tracked IME range presentation while the input controller owns composition state. */
export class CompositionPart extends EditorOverlayPart {
  private readonly model: TextModel;
  private compositionRange: TrackedRange | undefined;

  constructor(context: EditorViewContext, model: TextModel) {
    super(context);
    this.model = model;
    this.defer(() => this.compositionRange?.dispose());
  }

  public setRange(range: TextRange | undefined): void {
    const next = range
      ? this.model.trackRange(range, TrackedRangeStickiness.NeverGrowsAtEdges)
      : undefined;
    this.compositionRange?.dispose();
    this.compositionRange = next;
    this.render(this.context.layout);
  }

  public render(layout: EditorViewportLayout): void {
    const context = this.context.overlayContext(layout);
    if (!context) {
      return;
    }
    projectAsterCompositionOverlay(context, this.compositionRange?.range);
  }
}
