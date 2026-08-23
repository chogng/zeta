import "./linesDecorations.css";
import { type EditorViewportLayout } from "../../../common/viewLayout/editorViewportModel.js";
import { DecorationsPart } from "../decorations/decorationsPart.js";
import { EditorOverlayPart, EditorViewContext } from "../viewPart.js";
import { projectStanzaLinesDecorations } from "./linesDecorationsProjection.js";

/** Owns line-side decoration classes and tooltips for rendered logical lines. */
export class LinesDecorationsPart extends EditorOverlayPart {
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
		projectStanzaLinesDecorations(context, this.decorations.visibleDecorations(context));
	}
}
