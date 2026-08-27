import "./margin.css";
import { h } from "../../../../base/browser/dom.js";
import { FastDomNode } from "../../../../base/browser/fastDomNode.js";
import { toDisposable } from "../../../../base/common/lifecycle.js";
import { type TextModel } from "../../../common/model/textModel.js";
import { type EditorVisualLineProjection } from "../../../common/viewModel/modelLineProjection.js";
import { type TextMeasurer } from "../../config/fontMeasurements.js";
import { type RenderedLine } from "../viewLines/renderedLine.js";
import { EditorViewPart, type EditorRenderingContext } from "../../view/viewPart.js";
import { type EditorLineGutterRenderer } from "./lineGutterDecoration.js";

const GUTTER_HORIZONTAL_PADDING = 16;

export type MarginPresentation = "document" | "embedded";

export interface MarginPartOptions {
	readonly host: HTMLElement;
	readonly contentElement: HTMLElement;
	readonly model: TextModel;
	readonly textMeasurer: TextMeasurer;
	readonly presentation: MarginPresentation;
	readonly showLineNumbers: boolean;
	readonly lineGutterRenderer: EditorLineGutterRenderer | undefined;
	readonly readVisualProjection: () => EditorVisualLineProjection;
	readonly readRenderedLines: () => ReadonlyMap<number, RenderedLine>;
}

/** Owns the editor margin geometry, background, and feature-gutter projection. */
export class MarginPart extends EditorViewPart {
	readonly domNode: HTMLDivElement;
	private readonly root: FastDomNode<HTMLDivElement>;
	private readonly host: HTMLElement;
	private readonly contentElement: HTMLElement;
	private readonly model: TextModel;
	private readonly textMeasurer: TextMeasurer;
	private readonly presentation: MarginPresentation;
	private readonly showLineNumbers: boolean;
	private readonly lineGutterRenderer: EditorLineGutterRenderer | undefined;
	private readonly readVisualProjection: () => EditorVisualLineProjection;
	private readonly readRenderedLines: () => ReadonlyMap<number, RenderedLine>;

	constructor(options: MarginPartOptions) {
		super();
		this.host = options.host;
		this.contentElement = options.contentElement;
		this.model = options.model;
		this.textMeasurer = options.textMeasurer;
		this.presentation = options.presentation;
		this.showLineNumbers = options.showLineNumbers;
		this.lineGutterRenderer = options.lineGutterRenderer;
		this.readVisualProjection = options.readVisualProjection;
		this.readRenderedLines = options.readRenderedLines;
		const domNode = h(options.host.ownerDocument, "div");
		this._register(toDisposable(() => domNode.remove()));
		this.domNode = domNode;
		this.root = new FastDomNode(this.domNode);
		this.root.setClassName("stanza-editor-margin");
		this.domNode.setAttribute("role", "presentation");
		this.domNode.setAttribute("aria-hidden", "true");
	}

	get gutterWidth(): number {
		if (this.presentation === "embedded") return 0;
		return this.lineNumbersWidth + this.featureGutterWidth;
	}

	private get lineNumbersWidth(): number {
		if (this.presentation === "embedded" || !this.showLineNumbers) return 0;
		const digitCount = String(this.model.lineCount).length;
		return Math.ceil(this.textMeasurer.measureLineWidth("9".repeat(digitCount)) + GUTTER_HORIZONTAL_PADDING);
	}

	get featureGutterWidth(): number {
		return this.lineGutterRenderer?.width ?? 0;
	}

	get textLeft(): number {
		return this.gutterWidth + this.textMeasurer.contentLeftPadding;
	}

	render(context: EditorRenderingContext): void {
		const layout = context.layout;
		const gutterWidth = this.gutterWidth;
		const lineNumbersWidth = this.lineNumbersWidth;
		const featureGutterWidth = this.featureGutterWidth;
		this.root.setWidth(gutterWidth);
		this.root.setHeight(layout.contentSize.height);
		this.root.setHidden(gutterWidth === 0);
		for (const [visualLineIndex, line] of this.readRenderedLines()) {
			const visualLine = this.readVisualProjection().lineAt(visualLineIndex);
			if (!visualLine) continue;
			this.lineGutterRenderer?.render(line.featureGutterElement, visualLine.logicalLineIndex, visualLine.firstForLogicalLine);
		}
		this.host.style.setProperty("--stanza-editor-gutter-width", `${gutterWidth}px`);
		this.host.style.setProperty("--stanza-editor-line-numbers-width", `${lineNumbersWidth}px`);
		this.host.style.setProperty("--stanza-editor-feature-gutter-width", `${featureGutterWidth}px`);
		this.contentElement.style.setProperty("--stanza-editor-gutter-width", `${gutterWidth}px`);
		this.contentElement.style.setProperty("--stanza-editor-line-numbers-width", `${lineNumbersWidth}px`);
		this.contentElement.style.setProperty("--stanza-editor-feature-gutter-width", `${featureGutterWidth}px`);
	}
}
