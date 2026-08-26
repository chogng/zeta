import "./margin.css";
import { h } from "../../../../base/browser/dom.js";
import { FastDomNode } from "../../../../base/browser/fastDomNode.js";
import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import { type TextModel } from "../../../common/model/textModel.js";
import { type EditorViewportLayout } from "../../../common/viewLayout/editorViewportModel.js";
import { type EditorVisualLineProjection } from "../../../common/viewModel/modelLineProjection.js";
import { type TextMeasurer } from "../../config/fontMeasurements.js";
import { type RenderedLine } from "../viewLines/renderedLine.js";
import { type EditorViewPart } from "../viewPart.js";
import { type EditorLineGutterDecoration } from "./lineGutterDecoration.js";

const GUTTER_HORIZONTAL_PADDING = 16;

export type MarginPresentation = "document" | "embedded";

export interface MarginPartOptions {
	readonly host: HTMLElement;
	readonly contentElement: HTMLElement;
	readonly model: TextModel;
	readonly textMeasurer: TextMeasurer;
	readonly presentation: MarginPresentation;
	readonly showLineNumbers: boolean;
	readonly lineGutterDecoration: EditorLineGutterDecoration | undefined;
	readonly readVisualProjection: () => EditorVisualLineProjection;
	readonly readRenderedLines: () => ReadonlyMap<number, RenderedLine>;
}

/** Owns the editor margin geometry, background, and feature-gutter projection. */
export class MarginPart extends DisposableOwner implements EditorViewPart {
	readonly domNode: HTMLDivElement;
	private readonly root: FastDomNode<HTMLDivElement>;
	private readonly host: HTMLElement;
	private readonly contentElement: HTMLElement;
	private readonly model: TextModel;
	private readonly textMeasurer: TextMeasurer;
	private readonly presentation: MarginPresentation;
	private readonly showLineNumbers: boolean;
	private readonly lineGutterDecoration: EditorLineGutterDecoration | undefined;
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
		this.lineGutterDecoration = options.lineGutterDecoration;
		this.readVisualProjection = options.readVisualProjection;
		this.readRenderedLines = options.readRenderedLines;
		this.domNode = this.adopt(h(options.host.ownerDocument, "div"), domNode => domNode.remove());
		this.root = new FastDomNode(this.domNode);
		this.root.setClassName("stanza-editor-margin");
		this.domNode.setAttribute("role", "presentation");
		this.domNode.setAttribute("aria-hidden", "true");
	}

	get gutterWidth(): number {
		if (this.presentation === "embedded") return 0;
		if (!this.showLineNumbers) return this.featureGutterWidth;
		const digitCount = String(this.model.lineCount).length;
		return Math.ceil(
			this.textMeasurer.measureLineWidth("9".repeat(digitCount)) +
			GUTTER_HORIZONTAL_PADDING +
			this.additionalFeatureGutterWidth,
		);
	}

	get featureGutterWidth(): number {
		const decoration = this.lineGutterDecoration;
		if (!decoration) return 0;
		return "width" in decoration && typeof decoration.width === "number" ? decoration.width : 20;
	}

	get additionalFeatureGutterWidth(): number {
		return Math.max(0, this.featureGutterWidth - 20);
	}

	get textLeft(): number {
		return this.gutterWidth + this.textMeasurer.contentLeftPadding;
	}

	render(layout: EditorViewportLayout): void {
		const gutterWidth = this.gutterWidth;
		const featureGutterWidth = this.featureGutterWidth;
		const additionalFeatureGutterWidth = this.additionalFeatureGutterWidth;
		this.root.setWidth(gutterWidth);
		this.root.setHeight(layout.contentSize.height);
		this.root.setHidden(gutterWidth === 0);
		for (const [visualLineIndex, line] of this.readRenderedLines()) {
			const visualLine = this.readVisualProjection().lineAt(visualLineIndex);
			if (!visualLine) continue;
			this.lineGutterDecoration?.project(line.featureGutterElement, visualLine.logicalLineIndex, visualLine.firstForLogicalLine);
		}
		this.host.style.setProperty("--stanza-editor-gutter-width", `${gutterWidth}px`);
		this.host.style.setProperty("--stanza-editor-feature-gutter-width", `${featureGutterWidth}px`);
		this.host.style.setProperty("--stanza-editor-additional-feature-gutter-width", `${additionalFeatureGutterWidth}px`);
		this.contentElement.style.setProperty("--stanza-editor-gutter-width", `${gutterWidth}px`);
		this.contentElement.style.setProperty("--stanza-editor-feature-gutter-width", `${featureGutterWidth}px`);
		this.contentElement.style.setProperty("--stanza-editor-additional-feature-gutter-width", `${additionalFeatureGutterWidth}px`);
	}
}
