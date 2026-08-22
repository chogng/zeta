import "./selections.css";
import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import { type EditorSelectionController } from "../../../common/cursor/editorSelectionController.js";
import { type EditorViewportLayout } from "../../../common/viewLayout/editorViewportModel.js";
import { projectAsterCurrentLineHighlight, projectAsterSelectionOverlays } from "./selectionProjection.js";
import { type ViewportOverlayContext } from "../viewportOverlay/viewportOverlayPresentation.js";
import { type EditorViewPart } from "../viewPart.js";

export interface SelectionsPartOptions {
  readonly selectionController: EditorSelectionController | undefined;
  readonly readOverlayContext: (layout: EditorViewportLayout) => ViewportOverlayContext;
}

/** Projects selection ranges and current-line state without owning selection state. */
export class SelectionsPart extends DisposableOwner implements EditorViewPart {
  private readonly selectionController: EditorSelectionController | undefined;
  private readonly readOverlayContext: (layout: EditorViewportLayout) => ViewportOverlayContext;

  constructor(options: SelectionsPartOptions) {
    super();
    this.selectionController = options.selectionController;
    this.readOverlayContext = options.readOverlayContext;
  }

  render(layout: EditorViewportLayout): void {
    const context = this.readOverlayContext(layout);
    if (context.visualLineProjection.modelVersion !== context.model.version) return;
    projectAsterCurrentLineHighlight(context, this.selectionController);
    projectAsterSelectionOverlays(context, this.selectionController);
  }
}
