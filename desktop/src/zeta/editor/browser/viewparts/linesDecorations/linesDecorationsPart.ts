import "./linesDecorations.css";
import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import { type EditorViewportLayout } from "../../../common/viewLayout/editorViewportModel.js";
import { type ResolvedDecoration } from "../decorations/decorationPresentation.js";
import { type ViewportOverlayContext } from "../viewportOverlay/viewportOverlayPresentation.js";
import { type EditorViewPart } from "../viewPart.js";
import { projectAsterLinesDecorations } from "./linesDecorationsProjection.js";

export interface LinesDecorationsPartOptions {
  readonly readOverlayContext: (layout: EditorViewportLayout) => ViewportOverlayContext;
  readonly readDecorations: (layout: EditorViewportLayout) => readonly ResolvedDecoration[];
}

/** Owns line-side decoration classes and tooltips for rendered logical lines. */
export class LinesDecorationsPart extends DisposableOwner implements EditorViewPart {
  private readonly readOverlayContext: (layout: EditorViewportLayout) => ViewportOverlayContext;
  private readonly readDecorations: (layout: EditorViewportLayout) => readonly ResolvedDecoration[];

  constructor(options: LinesDecorationsPartOptions) {
    super();
    this.readOverlayContext = options.readOverlayContext;
    this.readDecorations = options.readDecorations;
  }

  render(layout: EditorViewportLayout): void {
    const context = this.readOverlayContext(layout);
    if (context.visualLineProjection.modelVersion !== context.model.version) return;
    projectAsterLinesDecorations(context, this.readDecorations(layout));
  }
}
