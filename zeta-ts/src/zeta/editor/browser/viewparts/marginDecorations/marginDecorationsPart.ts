import "./marginDecorations.css";
import { type EditorViewportLayout } from "../../../common/viewLayout/editorViewportModel.js";
import { DecorationsPart } from "../decorations/decorationsPart.js";
import { EditorOverlayPart, EditorViewContext } from "../viewPart.js";
import { projectStanzaDiagnosticMarginDecorations } from "./marginDecorationsProjection.js";

/** Projects line-level diagnostics into the editor margin. */
export class MarginDecorationsPart extends EditorOverlayPart {
	private readonly decorations: DecorationsPart;

	constructor(context: EditorViewContext, decorations: DecorationsPart) {
		super(context);
		this.decorations = decorations;
	}

	public render(layout: EditorViewportLayout): void {
		const context = this.context.overlayContext(layout);
		if (!context) {
			return;
		}
		projectStanzaDiagnosticMarginDecorations(context, this.decorations.visibleDecorations(context));
	}
}
