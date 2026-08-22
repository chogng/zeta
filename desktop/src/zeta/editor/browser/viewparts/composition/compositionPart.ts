import "./composition.css";
import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import { type TextRange } from "../../../common/core/text.js";
import { type TextModel } from "../../../common/model/textModel.js";
import { TrackedRangeStickiness, type TrackedRange } from "../../../common/model/trackedRange.js";
import { type EditorViewportLayout } from "../../../common/viewLayout/editorViewportModel.js";
import { type ViewportOverlayContext } from "../viewportOverlay/viewportOverlayPresentation.js";
import { projectAsterCompositionOverlay } from "./compositionProjection.js";
import { type EditorViewPart } from "../viewPart.js";

export interface CompositionPartOptions {
  readonly model: TextModel;
  readonly readLayout: () => EditorViewportLayout;
  readonly readOverlayContext: (layout: EditorViewportLayout) => ViewportOverlayContext;
}

/** Owns tracked IME range presentation while the input controller owns composition state. */
export class CompositionPart extends DisposableOwner implements EditorViewPart {
  private readonly model: TextModel;
  private readonly readLayout: () => EditorViewportLayout;
  private readonly readOverlayContext: (layout: EditorViewportLayout) => ViewportOverlayContext;
  private compositionRange: TrackedRange | undefined;

  constructor(options: CompositionPartOptions) {
    super();
    this.model = options.model;
    this.readLayout = options.readLayout;
    this.readOverlayContext = options.readOverlayContext;
    this.defer(() => this.compositionRange?.dispose());
  }

  setRange(range: TextRange | undefined): void {
    const next = range
      ? this.model.trackRange(range, TrackedRangeStickiness.NeverGrowsAtEdges)
      : undefined;
    this.compositionRange?.dispose();
    this.compositionRange = next;
    this.render(this.readLayout());
  }

  render(layout: EditorViewportLayout): void {
    const context = this.readOverlayContext(layout);
    if (context.visualLineProjection.modelVersion !== context.model.version) return;
    projectAsterCompositionOverlay(context, this.compositionRange?.range);
  }
}
