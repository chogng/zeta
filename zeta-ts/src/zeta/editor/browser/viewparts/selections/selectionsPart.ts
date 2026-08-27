import "./selections.css";
import { type EditorSelectionController } from "../../../common/cursor/editorSelectionController.js";
import { projectStanzaCurrentLineHighlight, projectStanzaSelectionOverlays } from "./selectionProjection.js";
import { DynamicViewOverlay } from "../../view/dynamicViewOverlay.js";
import { type EditorRenderingContext, EditorViewContext } from "../../view/viewPart.js";
import { ViewPartRows } from '../../view/viewPartRows.js';

/** Projects selection ranges and current-line state without owning selection state. */
export class SelectionsPart extends DynamicViewOverlay {
	public readonly domNode: HTMLElement;
	private readonly selectionController: EditorSelectionController | undefined;
	private readonly rows: ViewPartRows;

	constructor(context: EditorViewContext, host: HTMLElement, selectionController: EditorSelectionController | undefined) {
		super(context);
		this.rows = this._register(new ViewPartRows(host, 'stanza-editor-selections-layer', 'stanza-editor-line-selections'));
		this.domNode = this.rows.domNode;
		this.selectionController = selectionController;
	}

	public render(context: EditorRenderingContext): void {
		const overlay = context.overlay;
		if (!overlay) {
			return;
		}
		const rows = this.rows.render(context);
		projectStanzaCurrentLineHighlight(overlay, this.selectionController, rows);
		projectStanzaSelectionOverlays(overlay, this.selectionController, rows);
	}
}
