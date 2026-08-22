import "./viewCursors.css";
import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import { type EditorSelectionController } from "../../../common/cursor/editorSelectionController.js";
import { type EditorViewportLayout } from "../../../common/viewLayout/editorViewportModel.js";
import { projectAsterCursorOverlays } from "./cursorProjection.js";
import { type ViewportOverlayContext } from "../viewportOverlay/viewportOverlayPresentation.js";
import { type EditorViewPart } from "../viewPart.js";

export interface ViewCursorsPartOptions {
  readonly selectionController: EditorSelectionController | undefined;
  readonly readOverlayContext: (layout: EditorViewportLayout) => ViewportOverlayContext;
}

/** Projects primary and secondary carets without owning cursor positions. */
export class ViewCursorsPart extends DisposableOwner implements EditorViewPart {
  private readonly selectionController: EditorSelectionController | undefined;
  private readonly readOverlayContext: (layout: EditorViewportLayout) => ViewportOverlayContext;

  constructor(options: ViewCursorsPartOptions) {
    super();
    this.selectionController = options.selectionController;
    this.readOverlayContext = options.readOverlayContext;
  }

  render(layout: EditorViewportLayout): void {
    const context = this.readOverlayContext(layout);
    if (context.visualLineProjection.modelVersion !== context.model.version) return;
    projectAsterCursorOverlays(context, this.selectionController);
  }
}
