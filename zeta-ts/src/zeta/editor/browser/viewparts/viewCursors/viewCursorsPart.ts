import "./viewCursors.css";
import { type EditorSelectionController } from "../../../common/cursor/editorSelectionController.js";
import { projectStanzaCursorOverlays } from "./cursorProjection.js";
import { DynamicViewOverlay } from "../../view/dynamicViewOverlay.js";
import { type EditorRenderingContext, EditorViewContext } from "../../view/viewPart.js";

/** Projects primary and secondary carets without owning cursor positions. */
export class ViewCursorsPart extends DynamicViewOverlay {
	private readonly selectionController: EditorSelectionController | undefined;

	constructor(context: EditorViewContext, selectionController: EditorSelectionController | undefined) {
		super(context);
		this.selectionController = selectionController;
	}

	public render(context: EditorRenderingContext): void {
		const overlay = context.overlay;
		if (!overlay) {
			return;
		}
		projectStanzaCursorOverlays(overlay, this.selectionController);
	}
}
