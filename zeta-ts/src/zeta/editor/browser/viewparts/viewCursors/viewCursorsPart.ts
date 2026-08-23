import "./viewCursors.css";
import { type EditorSelectionController } from "../../../common/cursor/editorSelectionController.js";
import { type EditorViewportLayout } from "../../../common/viewLayout/editorViewportModel.js";
import { projectStanzaCursorOverlays } from "./cursorProjection.js";
import { EditorOverlayPart, EditorViewContext } from "../viewPart.js";

/** Projects primary and secondary carets without owning cursor positions. */
export class ViewCursorsPart extends EditorOverlayPart {
	private readonly selectionController: EditorSelectionController | undefined;

	constructor(context: EditorViewContext, selectionController: EditorSelectionController | undefined) {
		super(context);
		this.selectionController = selectionController;
	}

	public render(layout: EditorViewportLayout): void {
		const context = this.context.overlayContext(layout);
		if (!context) {
			return;
		}
		projectStanzaCursorOverlays(context, this.selectionController);
	}
}
