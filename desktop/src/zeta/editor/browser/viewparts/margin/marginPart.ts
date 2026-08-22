import "./margin.css";
import { h } from "../../../../base/browser/dom.js";
import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import { type TextModel } from "../../../common/model/textModel.js";
import { type EditorViewportLayout } from "../../../common/viewLayout/editorViewportModel.js";
import { type EditorVisualLineProjection } from "../../../common/viewModel/modelLineProjection.js";
import { type TextMeasurer } from "../../measurement/fontMetrics.js";
import { type RenderedLine } from "../viewLines/renderedLine.js";
import { type EditorViewPart } from "../viewPart.js";
import { type EditorLineGutterDecoration } from "./lineGutterDecoration.js";

const GUTTER_HORIZONTAL_PADDING = 16;

export type MarginPresentation = "document" | "embedded";

export interface MarginPartOptions {
  readonly host: HTMLElement;
  readonly container: HTMLElement;
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
  readonly element: HTMLDivElement;
  private readonly host: HTMLElement;
  private readonly container: HTMLElement;
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
    this.container = options.container;
    this.model = options.model;
    this.textMeasurer = options.textMeasurer;
    this.presentation = options.presentation;
    this.showLineNumbers = options.showLineNumbers;
    this.lineGutterDecoration = options.lineGutterDecoration;
    this.readVisualProjection = options.readVisualProjection;
    this.readRenderedLines = options.readRenderedLines;
    this.element = h(options.container.ownerDocument, "div");
    this.element.className = "aster-editor-margin";
    this.element.setAttribute("role", "presentation");
    this.element.setAttribute("aria-hidden", "true");
    options.container.append(this.element);
    this.defer(() => this.element.remove());
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
    this.element.style.width = `${gutterWidth}px`;
    this.element.style.height = `${layout.contentSize.height}px`;
    this.element.hidden = gutterWidth === 0;
    for (const [visualLineIndex, line] of this.readRenderedLines()) {
      const visualLine = this.readVisualProjection().lineAt(visualLineIndex);
      if (!visualLine) continue;
      this.lineGutterDecoration?.project(line.featureGutterElement, visualLine.logicalLineIndex, visualLine.firstForLogicalLine);
    }
    this.host.style.setProperty("--aster-editor-gutter-width", `${gutterWidth}px`);
    this.host.style.setProperty("--aster-editor-feature-gutter-width", `${featureGutterWidth}px`);
    this.host.style.setProperty("--aster-editor-additional-feature-gutter-width", `${additionalFeatureGutterWidth}px`);
    this.container.style.setProperty("--aster-editor-gutter-width", `${gutterWidth}px`);
    this.container.style.setProperty("--aster-editor-feature-gutter-width", `${featureGutterWidth}px`);
    this.container.style.setProperty("--aster-editor-additional-feature-gutter-width", `${additionalFeatureGutterWidth}px`);
  }
}
