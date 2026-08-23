import { findDocumentNode, type DocumentNode, type DocumentNodeId } from "./document.js";
import { type DocumentFragment } from "./documentSerialization.js";
import { type DocumentSchema } from "./documentSchema.js";
import { type DocumentSelection } from "../core/documentSelection.js";

/** Extracts a validated block fragment from a text selection. */
export function extractDocumentFragment(schema: DocumentSchema, document: DocumentNode, selection: DocumentSelection): DocumentFragment | undefined {
  if (selection.kind === "all") return Object.freeze({ content: Object.freeze([...document.content]) });
  if (selection.kind !== "text") return undefined;
  const anchor = findTextBlockLocation(document, selection.anchor.nodeId);
  const head = findTextBlockLocation(document, selection.head.nodeId);
  if (!anchor || !head) return undefined;
  if (anchor.block.id === head.block.id) {
    const forward = anchor.textIndex < head.textIndex || (anchor.textIndex === head.textIndex && selection.anchor.offset <= selection.head.offset);
    const start = forward ? { location: anchor, point: selection.anchor } : { location: head, point: selection.head };
    const end = forward ? { location: head, point: selection.head } : { location: anchor, point: selection.anchor };
    const content = sliceInlineContent(schema, start.location.block, start.location.textIndex, start.point.offset, end.location.textIndex, end.point.offset);
    if (content.length === 0) return undefined;
    return Object.freeze({ content: Object.freeze([createBlock(schema, start.location.block, content)]) });
  }
  if (anchor.parent.id !== head.parent.id) return undefined;
  const forward = anchor.parentIndex < head.parentIndex;
  const start = forward ? { location: anchor, point: selection.anchor } : { location: head, point: selection.head };
  const end = forward ? { location: head, point: selection.head } : { location: anchor, point: selection.anchor };
  const blocks: DocumentNode[] = [];
  for (let index = start.location.parentIndex; index <= end.location.parentIndex; index += 1) {
    const block = start.location.parent.content[index];
    if (!block || !isTextBlock(block)) return undefined;
    if (index === start.location.parentIndex) {
      const content = sliceInlineContent(schema, block, start.location.textIndex, start.point.offset, block.content.length - 1, Number.MAX_SAFE_INTEGER);
      blocks.push(createBlock(schema, block, content));
    } else if (index === end.location.parentIndex) {
      const content = sliceInlineContent(schema, block, 0, 0, end.location.textIndex, end.point.offset);
      blocks.push(createBlock(schema, block, content));
    } else {
      blocks.push(createBlock(schema, block, block.content));
    }
  }
  return Object.freeze({ content: Object.freeze(blocks) });
}

interface TextBlockLocation {
  readonly block: DocumentNode;
  readonly parent: DocumentNode;
  readonly parentIndex: number;
  readonly textIndex: number;
}

function findTextBlockLocation(root: DocumentNode, textNodeId: DocumentNodeId): TextBlockLocation | undefined {
  const textLocation = findDocumentNode(root, textNodeId);
  const block = textLocation?.parent;
  if (!block || !isTextBlock(block)) return undefined;
  const blockLocation = findDocumentNode(root, block.id);
  const textIndex = block.content.findIndex(child => child.id === textNodeId && child.text !== undefined);
  if (!blockLocation?.parent || textIndex < 0) return undefined;
  return { block, parent: blockLocation.parent, parentIndex: blockLocation.index, textIndex };
}

function sliceInlineContent(schema: DocumentSchema, block: DocumentNode, startIndex: number, startOffset: number, endIndex: number, endOffset: number): readonly DocumentNode[] {
  if (block.content.length === 0 || startIndex < 0 || endIndex < startIndex) return [];
  const content: DocumentNode[] = [];
  for (let index = startIndex; index <= endIndex; index += 1) {
    const child = block.content[index];
    if (!child) continue;
    if (child.text !== undefined) {
      const from = index === startIndex ? Math.max(0, Math.min(startOffset, child.text.length)) : 0;
      const to = index === endIndex ? Math.max(from, Math.min(endOffset, child.text.length)) : child.text.length;
      if (to > from) content.push(schema.createText(child.text.slice(from, to), { id: child.id, marks: child.marks }));
    } else if (index > startIndex || startOffset === 0) {
      content.push(schema.createNode(child.type, { id: child.id, attrs: child.attrs, content: child.content, marks: child.marks, ...(child.text === undefined ? {} : { text: child.text }) }));
    }
  }
  return content;
}

function createBlock(schema: DocumentSchema, source: DocumentNode, content: readonly DocumentNode[]): DocumentNode {
  return schema.createNode(source.type, { id: source.id, attrs: source.attrs, content });
}

function isTextBlock(node: DocumentNode): boolean {
  return node.type === "paragraph" || node.type === "heading" || node.type === "textBlock";
}
