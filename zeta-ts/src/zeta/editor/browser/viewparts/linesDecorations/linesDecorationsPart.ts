import "./linesDecorations.css";
import { DecorationsPart } from "../decorations/decorationsPart.js";
import { DynamicViewOverlay } from "../../view/dynamicViewOverlay.js";
import { type EditorRenderingContext, EditorViewContext } from "../../view/viewPart.js";
import { projectStanzaLinesDecorations } from "./linesDecorationsProjection.js";

/** Owns line-side decoration classes and tooltips for rendered logical lines. */
export class LinesDecorationsPart extends DynamicViewOverlay {
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
		projectStanzaLinesDecorations(overlay, this.decorations.visibleDecorations(overlay));
	}
}
