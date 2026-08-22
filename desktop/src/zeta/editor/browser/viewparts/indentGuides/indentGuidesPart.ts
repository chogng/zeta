import "./indentGuides.css";
import { h } from "../../../../base/browser/dom.js";
import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import { type EditorViewportLayout } from "../../../common/viewLayout/editorViewportModel.js";
import { createAsterIndentationGuides } from "./indentationGuides.js";
import { type ViewportOverlayContext } from "../viewportOverlay/viewportOverlayPresentation.js";
import { type EditorViewPart } from "../viewPart.js";

export interface IndentGuidesPartOptions {
  readonly showIndentationGuides: boolean;
  readonly tabSize: number;
  readonly readOverlayContext: (layout: EditorViewportLayout) => ViewportOverlayContext;
}

/** Projects indentation guides into the reusable rows owned by ViewLinesPart. */
export class IndentGuidesPart extends DisposableOwner implements EditorViewPart {
  private readonly showIndentationGuides: boolean;
  private readonly tabSize: number;
  private readonly readOverlayContext: (layout: EditorViewportLayout) => ViewportOverlayContext;

  constructor(options: IndentGuidesPartOptions) {
    super();
    this.showIndentationGuides = options.showIndentationGuides;
    this.tabSize = options.tabSize;
    this.readOverlayContext = options.readOverlayContext;
  }

  render(layout: EditorViewportLayout): void {
    const context = this.readOverlayContext(layout);
    if (context.visualLineProjection.modelVersion !== context.model.version) return;
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
