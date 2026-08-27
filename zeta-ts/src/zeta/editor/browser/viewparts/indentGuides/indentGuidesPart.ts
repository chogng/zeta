import "./indentGuides.css";
import { h } from "../../../../base/browser/dom.js";
import { createStanzaIndentationGuides } from "./indentationGuides.js";
import { DynamicViewOverlay } from "../../view/dynamicViewOverlay.js";
import { type EditorRenderingContext, EditorViewContext } from "../../view/viewPart.js";
import { ViewPartRows } from '../../view/viewPartRows.js';

interface IndentGuidesPartOptions {
	readonly host: HTMLElement;
	readonly showIndentationGuides: boolean;
	readonly tabSize: number;
}

/** Owns and projects the visible indentation-guide rows. */
export class IndentGuidesPart extends DynamicViewOverlay {
	public readonly domNode: HTMLElement;
	private readonly showIndentationGuides: boolean;
	private readonly tabSize: number;
	private readonly rows: ViewPartRows;

	constructor(context: EditorViewContext, options: IndentGuidesPartOptions) {
		super(context);
		this.rows = this._register(new ViewPartRows(options.host, 'stanza-editor-indent-guides-layer', 'stanza-editor-line-indent-guides'));
		this.domNode = this.rows.domNode;
		this.showIndentationGuides = options.showIndentationGuides;
		this.tabSize = options.tabSize;
	}

	public render(context: EditorRenderingContext): void {
		const overlay = context.overlay;
		if (!overlay) {
			return;
		}
		for (const [visualLineIndex, row] of this.rows.render(context)) {
			row.replaceChildren();
			if (!this.showIndentationGuides) continue;
			const visualLine = overlay.visualLineProjection.lineAt(visualLineIndex);
			if (!visualLine?.firstForLogicalLine) continue;
			const text = overlay.model.getLineContent(visualLine.logicalLineIndex);
			for (const guide of createStanzaIndentationGuides(text, this.tabSize)) {
				const element = h(overlay.ownerDocument, "span");
				element.className = "stanza-editor-indent-guide";
				element.dataset.indentLevel = String(guide.level);
				element.style.left = `${overlay.textLeft + overlay.textMeasurer.measureLineWidth(text.slice(0, guide.columnIndex)) - 1}px`;
				row.append(element);
			}
		}
	}
}
