import "./rulers.css";
import { h, reset, fragment as createFragment } from "../../../../base/browser/dom.js";
import { FastDomNode } from "../../../../base/browser/fastDomNode.js";
import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import { type EditorViewportLayout } from "../../../common/viewLayout/editorViewportModel.js";
import { type TextMeasurer } from "../../measurement/fontMetrics.js";
import { type EditorViewPart } from "../viewPart.js";

/** One 1-based editor column at which a vertical guide is rendered. */
export interface EditorRuler {
	readonly column: number;
	readonly color?: string;
}

export interface RulersPartOptions {
	readonly ownerDocument: Document;
	readonly textMeasurer: TextMeasurer;
	readonly readTextLeft: () => number;
	readonly rulers?: readonly EditorRuler[];
}

/** Projects configured column guides into the scrollable editor content. */
export class RulersPart extends DisposableOwner implements EditorViewPart {
	readonly domNode: HTMLDivElement;
	private readonly root: FastDomNode<HTMLDivElement>;
	private readonly textMeasurer: TextMeasurer;
	private readonly readTextLeft: () => number;
	private readonly rulers: readonly EditorRuler[];
	private readonly renderedRulers: FastDomNode<HTMLDivElement>[] = [];

	constructor(options: RulersPartOptions) {
		super();
		this.textMeasurer = options.textMeasurer;
		this.readTextLeft = options.readTextLeft;
		this.rulers = Object.freeze([...(options.rulers ?? [])].map(validateRuler));
		this.domNode = this.adopt(h(options.ownerDocument, "div"), domNode => domNode.remove());
		this.root = new FastDomNode(this.domNode);
		this.root.setClassName("aster-editor-rulers");
		this.domNode.setAttribute("role", "presentation");
		this.domNode.setAttribute("aria-hidden", "true");
	}

	render(layout: EditorViewportLayout): void {
		const height = Math.min(layout.contentSize.height, 1_000_000);
		this.root.setWidth(layout.contentSize.width);
		this.root.setHeight(height);
		if (this.renderedRulers.length !== this.rulers.length) {
			const fragment = createFragment(this.domNode.ownerDocument);
			this.renderedRulers.length = 0;
			for (const ruler of this.rulers) {
				const element = new FastDomNode(h(this.domNode.ownerDocument, "div"));
				element.setClassName("aster-editor-ruler");
				fragment.append(element.domNode);
				this.renderedRulers.push(element);
			}
			reset(this.domNode, fragment);
		}
		for (let index = 0; index < this.rulers.length; index += 1) {
			const ruler = this.rulers[index]!;
			const element = this.renderedRulers[index]!;
			element.setLeft(this.readTextLeft() + this.textMeasurer.measureLineWidth("0".repeat(ruler.column)));
			element.setHeight(height);
			element.setBoxShadow(ruler.color
				? `1px 0 0 0 ${ruler.color} inset`
				: "");
		}
	}
}

function validateRuler(ruler: EditorRuler): EditorRuler {
	if (!ruler || !Number.isSafeInteger(ruler.column) || ruler.column < 1) {
		throw new RangeError("Aster ruler columns must be positive safe integers");
	}
	if (ruler.color !== undefined && (typeof ruler.color !== "string" || ruler.color.trim().length === 0)) {
		throw new TypeError("Aster ruler colors must be non-empty strings");
	}
	return Object.freeze({
		column: ruler.column,
		...(ruler.color === undefined ? {} : { color: ruler.color }),
	});
}
