import "./selections.css";
import { type EditorSelectionController } from "../../../common/cursor/editorSelectionController.js";
import { type EditorViewportLayout } from "../../../common/viewLayout/editorViewportModel.js";
import { projectStanzaCurrentLineHighlight, projectStanzaSelectionOverlays } from "./selectionProjection.js";
import { EditorOverlayPart, EditorViewContext } from "../viewPart.js";

/** Projects selection ranges and current-line state without owning selection state. */
export class SelectionsPart extends EditorOverlayPart {
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
		projectStanzaCurrentLineHighlight(context, this.selectionController);
		projectStanzaSelectionOverlays(context, this.selectionController);
	}
}
