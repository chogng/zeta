export interface IndentationGuide {
  readonly columnIndex: number;
  readonly level: number;
}

/** Returns one guide at every complete visual indentation unit in leading whitespace. */
export function createAlphaIndentationGuides(text: string, tabSize: number): readonly IndentationGuide[] {
  if (typeof text !== "string") throw new TypeError("Alpha indentation guides require text");
  if (!Number.isSafeInteger(tabSize) || tabSize < 1) throw new RangeError("Alpha indentation guide tab size must be a positive safe integer");
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
