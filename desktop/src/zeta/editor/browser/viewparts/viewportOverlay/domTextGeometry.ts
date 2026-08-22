export interface DomTextRectangle {
  readonly left: number;
  readonly width: number;
}

interface TextSegment {
  readonly node: Text;
  readonly startOffset: number;
  readonly endOffset: number;
}

interface CaretPosition {
  readonly offsetNode: Node;
  readonly offset: number;
}

interface CaretRange {
  readonly startContainer: Node;
  readonly startOffset: number;
}

interface CaretDocument {
  caretPositionFromPoint?(x: number, y: number): CaretPosition | null;
  caretRangeFromPoint?(x: number, y: number): CaretRange | null;
}

/**
 * Resolves logical UTF-16 offsets against one rendered Aster text fragment.
 *
 * Semantic-token spans may split source text into several DOM text nodes, but
 * this adapter deliberately exposes only the original contiguous offset space.
 */
export function createAsterDomTextRange(element: HTMLElement, startOffset: number, endOffset: number): Range | undefined {
  const segments = textSegments(element);
  const textLength = segments.at(-1)?.endOffset ?? 0;
  if (!isOffset(startOffset, textLength) || !isOffset(endOffset, textLength) || endOffset < startOffset) {
    throw new RangeError("Aster DOM text range offsets must be ordered UTF-16 positions");
  }
  const start = resolveBoundary(segments, startOffset);
  const end = resolveBoundary(segments, endOffset);
  if (!start || !end) return undefined;
  const range = element.ownerDocument.createRange();
  range.setStart(start.node, start.offset);
  range.setEnd(end.node, end.offset);
  return range;
}

/** Returns browser-shaped visual rectangles for one source range, if layout is available. */
export function getAsterDomTextRangeRectangles(element: HTMLElement, startOffset: number, endOffset: number, relativeTo: HTMLElement): readonly DomTextRectangle[] | undefined {
  const range = createAsterDomTextRange(element, startOffset, endOffset);
  if (!range || startOffset === endOffset) return undefined;
  if (typeof range.getClientRects !== "function") return undefined;
  const origin = relativeTo.getBoundingClientRect();
  const rectangles = [...range.getClientRects()]
    .filter(rectangle => Number.isFinite(rectangle.left) && Number.isFinite(rectangle.width) && rectangle.width > 0)
    .map(rectangle => Object.freeze({
      left: rectangle.left - origin.left,
      width: rectangle.width,
    }));
  return rectangles.length > 0 ? Object.freeze(rectangles) : undefined;
}

/** Returns the browser-shaped caret x-coordinate for one source offset, if layout is available. */
export function getAsterDomTextCaretLeft(element: HTMLElement, offset: number, relativeTo: HTMLElement): number | undefined {
  const range = createAsterDomTextRange(element, offset, offset);
  if (!range) return undefined;
  if (typeof range.getBoundingClientRect !== "function") return undefined;
  const origin = relativeTo.getBoundingClientRect();
  if (origin.width <= 0 && origin.height <= 0) return undefined;
  const rectangle = range.getBoundingClientRect();
  if (!Number.isFinite(rectangle.left) || (!Number.isFinite(rectangle.width) && rectangle.width !== 0)) return undefined;
  const left = rectangle.left - origin.left;
  return Number.isFinite(left) ? left : undefined;
}

/** Resolves a browser caret hit inside one rendered text fragment to its source offset. */
export function getAsterDomTextOffsetAtClientPoint(element: HTMLElement, clientX: number, clientY: number): number | undefined {
  if (!Number.isFinite(clientX) || !Number.isFinite(clientY)) {
    throw new RangeError("Aster DOM hit-test coordinates must be finite");
  }
  const document = element.ownerDocument as unknown as CaretDocument;
  const position = document.caretPositionFromPoint?.(clientX, clientY) ?? document.caretRangeFromPoint?.(clientX, clientY);
  if (!position) return undefined;
  const node = "offsetNode" in position ? position.offsetNode : position.startContainer;
  const offset = "offsetNode" in position ? position.offset : position.startOffset;
  if (!element.contains(node)) return undefined;
  return offsetForDomPosition(textSegments(element), node, offset);
}

function textSegments(element: HTMLElement): readonly TextSegment[] {
  const segments: TextSegment[] = [];
  const walker = element.ownerDocument.createTreeWalker(element, 4);
  let offset = 0;
  for (let node = walker.nextNode(); node; node = walker.nextNode()) {
    const text = node as Text;
    const length = text.data.length;
    segments.push(Object.freeze({ node: text, startOffset: offset, endOffset: offset + length }));
    offset += length;
  }
  return segments;
}

function resolveBoundary(segments: readonly TextSegment[], offset: number): { readonly node: Text; readonly offset: number } | undefined {
  if (segments.length === 0) return undefined;
  const segment = offset === segments.at(-1)?.endOffset
    ? segments.at(-1)
    : segments.find(candidate => offset >= candidate.startOffset && offset < candidate.endOffset);
  if (!segment) return undefined;
  return Object.freeze({ node: segment.node, offset: offset - segment.startOffset });
}

function offsetForDomPosition(segments: readonly TextSegment[], node: Node, offset: number): number | undefined {
  if (!Number.isSafeInteger(offset) || offset < 0) return undefined;
  if (node.nodeType === node.TEXT_NODE) {
    const segment = segments.find(candidate => candidate.node === node);
    if (!segment || offset > segment.node.data.length) return undefined;
    return segment.startOffset + offset;
  }
  if (node.nodeType !== node.ELEMENT_NODE || offset > node.childNodes.length) return undefined;
  const child = node.childNodes[offset] ?? node.childNodes[offset - 1];
  if (!child) return segments.length === 0 && offset === 0 ? 0 : undefined;
  const segment = segments.find(candidate => child.contains(candidate.node));
  if (!segment) return undefined;
  return child === node.childNodes[offset] ? segment.startOffset : segment.endOffset;
}

function isOffset(value: number, textLength: number): boolean {
  return Number.isSafeInteger(value) && value >= 0 && value <= textLength;
}
