import "./indentGuides.css";
import { h } from "../../../../base/browser/dom.js";
import { DynamicViewOverlay } from "../../view/dynamicViewOverlay.js";
import { type EditorRenderingContext, EditorViewContext } from "../../view/viewPart.js";
import { ViewPartRows } from '../../view/viewPartRows.js';

interface IndentGuidesOptions {
	readonly host: HTMLElement;
	readonly showIndentationGuides: boolean;
	readonly tabSize: number;
}

/** Owns and projects the visible indentation-guide rows. */
export class IndentGuidesOverlay extends DynamicViewOverlay {
	public readonly domNode: HTMLElement;
	private readonly showIndentationGuides: boolean;
	private readonly tabSize: number;
	private readonly rows: ViewPartRows;

	constructor(context: EditorViewContext, options: IndentGuidesOptions) {
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

export interface IndentationGuide {
	readonly columnIndex: number;
	readonly level: number;
}

/** Returns one guide at every complete visual indentation unit in leading whitespace. */
export function createStanzaIndentationGuides(text: string, tabSize: number): readonly IndentationGuide[] {
	if (typeof text !== "string") throw new TypeError("Stanza indentation guides require text");
	if (!Number.isSafeInteger(tabSize) || tabSize < 1) throw new RangeError("Stanza indentation guide tab size must be a positive safe integer");
	const guides: IndentationGuide[] = [];
	let visualColumn = 0;
	for (let columnIndex = 0; columnIndex < text.length; columnIndex += 1) {
		const character = text[columnIndex]!;
		if (character !== " " && character !== "\t") break;
		visualColumn = character === "\t"
			? visualColumn + tabSize - (visualColumn % tabSize)
			: visualColumn + 1;
		if (visualColumn % tabSize === 0) {
			guides.push(Object.freeze({
				columnIndex: columnIndex + 1,
				level: visualColumn / tabSize,
			}));
		}
	}
	return Object.freeze(guides);
}
