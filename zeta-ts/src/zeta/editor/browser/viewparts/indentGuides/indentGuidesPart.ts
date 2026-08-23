import "./indentGuides.css";
import { h } from "../../../../base/browser/dom.js";
import { type EditorViewportLayout } from "../../../common/viewLayout/editorViewportModel.js";
import { createAsterIndentationGuides } from "./indentationGuides.js";
import { EditorOverlayPart, EditorViewContext } from "../viewPart.js";

interface IndentGuidesPartOptions {
	readonly showIndentationGuides: boolean;
	readonly tabSize: number;
}

/** Projects indentation guides into the reusable rows owned by ViewLinesPart. */
export class IndentGuidesPart extends EditorOverlayPart {
	private readonly showIndentationGuides: boolean;
	private readonly tabSize: number;

	constructor(context: EditorViewContext, options: IndentGuidesPartOptions) {
		super(context);
		this.showIndentationGuides = options.showIndentationGuides;
		this.tabSize = options.tabSize;
	}

	public render(layout: EditorViewportLayout): void {
		const context = this.context.overlayContext(layout);
		if (!context) {
			return;
		}
		for (const [visualLineIndex, line] of context.renderedLines) {
			line.indentationElement.replaceChildren();
			if (!this.showIndentationGuides) continue;
			const visualLine = context.visualLineProjection.lineAt(visualLineIndex);
			if (!visualLine?.firstForLogicalLine) continue;
			const text = context.model.getLineContent(visualLine.logicalLineIndex);
			for (const guide of createAsterIndentationGuides(text, this.tabSize)) {
				const element = h(context.ownerDocument, "span");
				element.className = "aster-editor-indent-guide";
				element.dataset.indentLevel = String(guide.level);
				element.style.left = `${context.textLeft + context.textMeasurer.measureLineWidth(text.slice(0, guide.columnIndex)) - 1}px`;
				line.indentationElement.append(element);
			}
		}
	}
}
