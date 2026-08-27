import "./viewCursors.css";
import { type EditorSelectionController } from "../../../common/cursor/editorSelectionController.js";
import { projectStanzaCursorOverlays } from "./cursorProjection.js";
import { DynamicViewOverlay } from "../../view/dynamicViewOverlay.js";
import { type EditorRenderingContext, EditorViewContext } from "../../view/viewPart.js";
import { ViewPartRows } from '../../view/viewPartRows.js';

/** Projects primary and secondary carets without owning cursor positions. */
export class ViewCursorsPart extends DynamicViewOverlay {
	public readonly domNode: HTMLElement;
	private readonly selectionController: EditorSelectionController | undefined;
	private readonly rows: ViewPartRows;

	constructor(context: EditorViewContext, host: HTMLElement, selectionController: EditorSelectionController | undefined) {
		super(context);
		this.rows = this._register(new ViewPartRows(host, 'stanza-editor-cursors-layer', 'stanza-editor-line-cursors'));
		this.domNode = this.rows.domNode;
		this.selectionController = selectionController;
	}

	public render(context: EditorRenderingContext): void {
		const overlay = context.overlay;
		if (!overlay) {
			return;
		}
		projectStanzaCursorOverlays(overlay, this.selectionController, this.rows.render(context));
	}
}
