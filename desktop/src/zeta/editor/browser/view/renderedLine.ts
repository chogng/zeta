import { type EditorLineGutterDecoration } from "./lineGutterDecoration.js";

export interface RenderedLine {
  readonly element: HTMLDivElement;
  readonly numberElement: HTMLSpanElement;
  readonly featureGutterElement: HTMLElement;
  readonly diagnosticElement: HTMLSpanElement;
  readonly textElement: HTMLSpanElement;
  readonly indentationElement: HTMLDivElement;
  readonly decorationElement: HTMLDivElement;
  readonly compositionElement: HTMLDivElement;
  readonly selectionElement: HTMLDivElement;
}

/** Creates one reusable virtual-line DOM subtree owned by Aster. */
export function createAsterRenderedLine(ownerDocument: Document, lineIndex: number, gutterDecoration: EditorLineGutterDecoration | undefined): RenderedLine {
  const element = ownerDocument.createElement("div");
  const numberElement = ownerDocument.createElement("span");
  const featureGutterElement = gutterDecoration?.create(ownerDocument) ?? ownerDocument.createElement("span");
  if (!gutterDecoration) featureGutterElement.hidden = true;
  const diagnosticElement = ownerDocument.createElement("span");
  const textElement = ownerDocument.createElement("span");
  const indentationElement = ownerDocument.createElement("div");
  const decorationElement = ownerDocument.createElement("div");
  const compositionElement = ownerDocument.createElement("div");
  const selectionElement = ownerDocument.createElement("div");
  element.className = "aster-editor-line";
  element.dataset.lineIndex = String(lineIndex);
  numberElement.className = "aster-editor-line-number";
  numberElement.setAttribute("aria-hidden", "true");
  diagnosticElement.className = "aster-editor-diagnostic-marker";
  diagnosticElement.hidden = true;
  diagnosticElement.setAttribute("aria-hidden", "true");
  textElement.className = "aster-editor-line-text";
  indentationElement.className = "aster-editor-line-indent-guides";
  indentationElement.setAttribute("aria-hidden", "true");
  decorationElement.className = "aster-editor-line-decorations";
  decorationElement.setAttribute("aria-hidden", "true");
  compositionElement.className = "aster-editor-line-composition";
  compositionElement.setAttribute("aria-hidden", "true");
  selectionElement.className = "aster-editor-line-selections";
  selectionElement.setAttribute("aria-hidden", "true");
  element.append(indentationElement, decorationElement, selectionElement, compositionElement, featureGutterElement, diagnosticElement, numberElement, textElement);
  return {
    element,
    numberElement,
    featureGutterElement,
    diagnosticElement,
    textElement,
    indentationElement,
    decorationElement,
    compositionElement,
    selectionElement,
  };
}
