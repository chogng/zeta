import "./indentGuides.css";
import { h } from "../../../../base/browser/dom.js";
import { createStanzaIndentationGuides } from "./indentationGuides.js";
import { DynamicViewOverlay } from "../../view/dynamicViewOverlay.js";
import { type EditorRenderingContext, EditorViewContext } from "../../view/viewPart.js";

interface IndentGuidesPartOptions {
	readonly showIndentationGuides: boolean;
	readonly tabSize: number;
}

/** Projects indentation guides into the reusable rows owned by ViewLayer. */
export class IndentGuidesPart extends DynamicViewOverlay {
	private readonly showIndentationGuides: boolean;
	private readonly tabSize: number;

	constructor(context: EditorViewContext, options: IndentGuidesPartOptions) {
		super(context);
		this.showIndentationGuides = options.showIndentationGuides;
		this.tabSize = options.tabSize;
	}

	public render(context: EditorRenderingContext): void {
		const overlay = context.overlay;
		if (!overlay) {
			return;
		}
		for (const [visualLineIndex, line] of overlay.renderedLines) {
			line.indentationElement.replaceChildren();
			if (!this.showIndentationGuides) continue;
			const visualLine = overlay.visualLineProjection.lineAt(visualLineIndex);
			if (!visualLine?.firstForLogicalLine) continue;
			const text = overlay.model.getLineContent(visualLine.logicalLineIndex);
			for (const guide of createStanzaIndentationGuides(text, this.tabSize)) {
				const element = h(overlay.ownerDocument, "span");
				element.className = "stanza-editor-indent-guide";
				element.dataset.indentLevel = String(guide.level);
				element.style.left = `${overlay.textLeft + overlay.textMeasurer.measureLineWidth(text.slice(0, guide.columnIndex)) - 1}px`;
				line.indentationElement.append(element);
			}
		}
	}
}
