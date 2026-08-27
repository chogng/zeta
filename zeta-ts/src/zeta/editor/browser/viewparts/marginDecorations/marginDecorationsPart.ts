import "./marginDecorations.css";
import { DecorationsPart } from "../decorations/decorationsPart.js";
import { DynamicViewOverlay } from "../../view/dynamicViewOverlay.js";
import { type EditorRenderingContext, EditorViewContext } from "../../view/viewPart.js";
import { projectStanzaDiagnosticMarginDecorations } from "./marginDecorationsProjection.js";
import { ViewPartRows } from '../../view/viewPartRows.js';

/** Projects line-level diagnostics into the editor margin. */
export class MarginDecorationsPart extends DynamicViewOverlay {
	public readonly domNode: HTMLElement;
	private readonly decorations: DecorationsPart;
	private readonly rows: ViewPartRows;

	constructor(context: EditorViewContext, host: HTMLElement, decorations: DecorationsPart) {
		super(context);
		this.rows = this._register(new ViewPartRows(host, 'stanza-editor-margin-decorations-layer', 'stanza-editor-diagnostic-marker'));
		this.domNode = this.rows.domNode;
		this.decorations = decorations;
	}

	public render(context: EditorRenderingContext): void {
		const overlay = context.overlay;
		if (!overlay) {
			return;
		}
		projectStanzaDiagnosticMarginDecorations(overlay, this.decorations.visibleDecorations(overlay), this.rows.render(context));
	}
}
