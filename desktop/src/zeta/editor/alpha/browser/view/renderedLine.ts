import { createAlphaFoldingDecoration } from "../../contrib/folding/browser/foldingDecorations.js";

export interface AlphaRenderedLine {
  readonly element: HTMLDivElement;
  readonly numberElement: HTMLSpanElement;
  readonly foldingElement: HTMLButtonElement;
  readonly diagnosticElement: HTMLSpanElement;
  readonly textElement: HTMLSpanElement;
  readonly indentationElement: HTMLDivElement;
  readonly decorationElement: HTMLDivElement;
  readonly compositionElement: HTMLDivElement;
  readonly selectionElement: HTMLDivElement;
}

/** Creates one reusable virtual-line DOM subtree owned by Alpha. */
export function createAlphaRenderedLine(ownerDocument: Document, lineIndex: number): AlphaRenderedLine {
  const element = ownerDocument.createElement("div");
  const numberElement = ownerDocument.createElement("span");
  const foldingElement = createAlphaFoldingDecoration(ownerDocument);
  const diagnosticElement = ownerDocument.createElement("span");
  const textElement = ownerDocument.createElement("span");
  const indentationElement = ownerDocument.createElement("div");
  const decorationElement = ownerDocument.createElement("div");
  const compositionElement = ownerDocument.createElement("div");
  const selectionElement = ownerDocument.createElement("div");
  element.className = "zeta-alpha-editor-line";
  element.dataset.lineIndex = String(lineIndex);
  numberElement.className = "zeta-alpha-editor-line-number";
  numberElement.setAttribute("aria-hidden", "true");
  diagnosticElement.className = "zeta-alpha-editor-diagnostic-marker";
  diagnosticElement.hidden = true;
  diagnosticElement.setAttribute("aria-hidden", "true");
  textElement.className = "zeta-alpha-editor-line-text";
  indentationElement.className = "zeta-alpha-editor-line-indent-guides";
  indentationElement.setAttribute("aria-hidden", "true");
  decorationElement.className = "zeta-alpha-editor-line-decorations";
  decorationElement.setAttribute("aria-hidden", "true");
  compositionElement.className = "zeta-alpha-editor-line-composition";
  compositionElement.setAttribute("aria-hidden", "true");
  selectionElement.className = "zeta-alpha-editor-line-selections";
  selectionElement.setAttribute("aria-hidden", "true");
  element.append(indentationElement, decorationElement, selectionElement, compositionElement, foldingElement, diagnosticElement, numberElement, textElement);
  return {
    element,
    numberElement,
    foldingElement,
    diagnosticElement,
    textElement,
    indentationElement,
    decorationElement,
    compositionElement,
    selectionElement,
  };
}
