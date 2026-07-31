export interface AlphaRenderedLine {
  readonly element: HTMLDivElement;
  readonly numberElement: HTMLSpanElement;
  readonly textElement: HTMLSpanElement;
  readonly decorationElement: HTMLDivElement;
  readonly compositionElement: HTMLDivElement;
  readonly selectionElement: HTMLDivElement;
}

/** Creates one reusable virtual-line DOM subtree owned by Alpha. */
export function createAlphaRenderedLine(ownerDocument: Document, lineIndex: number): AlphaRenderedLine {
  const element = ownerDocument.createElement("div");
  const numberElement = ownerDocument.createElement("span");
  const textElement = ownerDocument.createElement("span");
  const decorationElement = ownerDocument.createElement("div");
  const compositionElement = ownerDocument.createElement("div");
  const selectionElement = ownerDocument.createElement("div");
  element.className = "zeta-alpha-editor-line";
  element.dataset.lineIndex = String(lineIndex);
  numberElement.className = "zeta-alpha-editor-line-number";
  numberElement.setAttribute("aria-hidden", "true");
  textElement.className = "zeta-alpha-editor-line-text";
  decorationElement.className = "zeta-alpha-editor-line-decorations";
  decorationElement.setAttribute("aria-hidden", "true");
  compositionElement.className = "zeta-alpha-editor-line-composition";
  compositionElement.setAttribute("aria-hidden", "true");
  selectionElement.className = "zeta-alpha-editor-line-selections";
  selectionElement.setAttribute("aria-hidden", "true");
  element.append(decorationElement, selectionElement, compositionElement, numberElement, textElement);
  return {
    element,
    numberElement,
    textElement,
    decorationElement,
    compositionElement,
    selectionElement,
  };
}
