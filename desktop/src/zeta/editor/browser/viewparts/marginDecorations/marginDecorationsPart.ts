import "./marginDecorations.css";
import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import { type EditorViewportLayout } from "../../../common/viewLayout/editorViewportModel.js";
import { type ResolvedDecoration } from "../decorations/decorationPresentation.js";
import { type ViewportOverlayContext } from "../viewportOverlay/viewportOverlayPresentation.js";
import { type EditorViewPart } from "../viewPart.js";
import { projectAsterDiagnosticMarginDecorations } from "./marginDecorationsProjection.js";

export interface MarginDecorationsPartOptions {
  readonly readOverlayContext: (layout: EditorViewportLayout) => ViewportOverlayContext;
  readonly readDecorations: (layout: EditorViewportLayout) => readonly ResolvedDecoration[];
}

/** Projects line-level diagnostics into the editor margin. */
export class MarginDecorationsPart extends DisposableOwner implements EditorViewPart {
  private readonly readOverlayContext: (layout: EditorViewportLayout) => ViewportOverlayContext;
  private readonly readDecorations: (layout: EditorViewportLayout) => readonly ResolvedDecoration[];

  constructor(options: MarginDecorationsPartOptions) {
    super();
    this.readOverlayContext = options.readOverlayContext;
    this.readDecorations = options.readDecorations;
  }

  render(layout: EditorViewportLayout): void {
    const context = this.readOverlayContext(layout);
    if (context.visualLineProjection.modelVersion !== context.model.version) return;
    projectAsterDiagnosticMarginDecorations(context, this.readDecorations(layout));
  }
}
