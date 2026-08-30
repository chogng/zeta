import "./margin.css";
import { h } from "../../../../base/browser/dom.js";
import { FastDomNode } from "../../../../base/browser/fastDomNode.js";
import { toDisposable } from "../../../../base/common/lifecycle.js";
import { type TextModel } from "../../../common/model/textModel.js";
import { type TextMeasurer } from "../../config/fontMeasurements.js";
import { EditorViewPart, type EditorRenderingContext } from "../../view/viewPart.js";

const GUTTER_HORIZONTAL_PADDING = 16;

export type MarginPresentation = "document" | "embedded";

export interface MarginOptions {
	readonly host: HTMLElement;
	readonly contentElement: HTMLElement;
	readonly model: TextModel;
	readonly textMeasurer: TextMeasurer;
	readonly presentation: MarginPresentation;
	readonly showLineNumbers: boolean;
	readonly glyphMarginLaneCount: number;
	readonly lineHeight: number;
	readonly lineDecorationsWidth: number;
}

/** Owns editor margin geometry and its background. */
export class Margin extends EditorViewPart {
	public static readonly CLASS_NAME = 'glyph-margin';
	readonly domNode: HTMLDivElement;
	private readonly root: FastDomNode<HTMLDivElement>;
	private readonly host: HTMLElement;
	private readonly contentElement: HTMLElement;
	private readonly model: TextModel;
	private readonly textMeasurer: TextMeasurer;
	private readonly presentation: MarginPresentation;
	private readonly showLineNumbers: boolean;
	private readonly glyphMarginLaneCount: number;
	private readonly lineDecorationsWidth: number;
	private lineHeight: number;

	constructor(options: MarginOptions) {
		super();
		this.host = options.host;
		this.contentElement = options.contentElement;
		this.model = options.model;
		this.textMeasurer = options.textMeasurer;
		this.presentation = options.presentation;
		this.showLineNumbers = options.showLineNumbers;
		this.glyphMarginLaneCount = options.glyphMarginLaneCount;
		this.lineHeight = options.lineHeight;
		this.lineDecorationsWidth = options.lineDecorationsWidth;
		const domNode = h(options.host.ownerDocument, "div");
		this._register(toDisposable(() => domNode.remove()));
		this.domNode = domNode;
		this.root = new FastDomNode(this.domNode);
		this.root.setClassName(Margin.CLASS_NAME);
		this.domNode.setAttribute("role", "presentation");
		this.domNode.setAttribute("aria-hidden", "true");
	}

	get gutterWidth(): number {
		if (this.presentation === "embedded") return 0;
		return this.glyphMarginWidth + this.lineNumbersWidth + this.lineDecorationsWidth;
	}

	get glyphMarginLaneWidth(): number {
		return this.lineHeight;
	}

	private get glyphMarginWidth(): number {
		return this.glyphMarginLaneCount * this.glyphMarginLaneWidth;
	}

	private get lineNumbersWidth(): number {
		if (this.presentation === "embedded" || !this.showLineNumbers) return 0;
		const digitCount = String(this.model.lineCount).length;
		return Math.ceil(this.textMeasurer.measureLineWidth("9".repeat(digitCount)) + GUTTER_HORIZONTAL_PADDING);
	}

	get glyphMarginLeft(): number {
		return 0;
	}

	private get lineNumbersLeft(): number {
		return this.glyphMarginWidth;
	}

	private get lineDecorationsLeft(): number {
		return this.glyphMarginWidth + this.lineNumbersWidth;
	}

	get textLeft(): number {
		return this.gutterWidth + this.textMeasurer.contentLeftPadding;
	}

	setLineHeight(lineHeight: number): void {
		this.lineHeight = lineHeight;
	}

	render(context: EditorRenderingContext): void {
		const layout = context.layout;
		const gutterWidth = this.gutterWidth;
		const lineNumbersWidth = this.lineNumbersWidth;
		this.root.setWidth(gutterWidth);
		this.root.setHeight(layout.contentSize.height);
		const hidden = gutterWidth === 0;
		if (this.domNode.hidden !== hidden) this.domNode.hidden = hidden;
		this.host.style.setProperty("--stanza-editor-gutter-width", `${gutterWidth}px`);
		this.host.style.setProperty("--stanza-editor-line-numbers-width", `${lineNumbersWidth}px`);
		this.host.style.setProperty("--stanza-editor-glyph-margin-width", `${this.glyphMarginWidth}px`);
		this.host.style.setProperty("--stanza-editor-line-numbers-left", `${this.lineNumbersLeft}px`);
		this.host.style.setProperty("--stanza-editor-line-decorations-left", `${this.lineDecorationsLeft}px`);
		this.host.style.setProperty("--stanza-editor-line-decorations-width", `${this.lineDecorationsWidth}px`);
		this.contentElement.style.setProperty("--stanza-editor-gutter-width", `${gutterWidth}px`);
		this.contentElement.style.setProperty("--stanza-editor-line-numbers-width", `${lineNumbersWidth}px`);
		this.contentElement.style.setProperty("--stanza-editor-glyph-margin-width", `${this.glyphMarginWidth}px`);
		this.contentElement.style.setProperty("--stanza-editor-line-numbers-left", `${this.lineNumbersLeft}px`);
		this.contentElement.style.setProperty("--stanza-editor-line-decorations-left", `${this.lineDecorationsLeft}px`);
		this.contentElement.style.setProperty("--stanza-editor-line-decorations-width", `${this.lineDecorationsWidth}px`);
	}
}
