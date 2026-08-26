import "./marginDecorations.css";
import { DecorationsPart } from "../decorations/decorationsPart.js";
import { DynamicViewOverlay } from "../../view/dynamicViewOverlay.js";
import { type EditorRenderingContext, EditorViewContext } from "../../view/viewPart.js";
import { projectStanzaDiagnosticMarginDecorations } from "./marginDecorationsProjection.js";

/** Projects line-level diagnostics into the editor margin. */
export class MarginDecorationsPart extends DynamicViewOverlay {
	private readonly decorations: DecorationsPart;

	constructor(context: EditorViewContext, decorations: DecorationsPart) {
		super(context);
		this.decorations = decorations;
	}

	public render(context: EditorRenderingContext): void {
		const overlay = context.overlay;
		if (!overlay) {
			return;
		}
		projectStanzaDiagnosticMarginDecorations(overlay, this.decorations.visibleDecorations(overlay));
	}
}
