import "./selections.css";
import { type EditorSelectionController } from "../../../common/cursor/editorSelectionController.js";
import { projectStanzaCurrentLineHighlight, projectStanzaSelectionOverlays } from "./selectionProjection.js";
import { DynamicViewOverlay } from "../../view/dynamicViewOverlay.js";
import { type EditorRenderingContext, EditorViewContext } from "../../view/viewPart.js";

/** Projects selection ranges and current-line state without owning selection state. */
export class SelectionsPart extends DynamicViewOverlay {
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
		projectStanzaCurrentLineHighlight(overlay, this.selectionController);
		projectStanzaSelectionOverlays(overlay, this.selectionController);
	}
}
