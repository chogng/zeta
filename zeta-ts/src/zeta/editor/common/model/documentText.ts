import { findDocumentNode, type DocumentNode, type DocumentNodeId } from "./document.js";
import { type DocumentSchema } from "./documentSchema.js";
import { type DocumentSelection } from "../core/documentSelection.js";

/** Converts legacy plain text into a valid paragraph document for migration. */
export function documentFromPlainText(schema: DocumentSchema, text: string, documentId = "document-1"): DocumentNode {
	if (typeof text !== "string") throw new TypeError("Plain document text must be a string");
	const lines = text.replaceAll("\r\n", "\n").replaceAll("\r", "\n").split("\n");
	const paragraphs = lines.map((line, index) => schema.createNode("paragraph", {
		id: `${documentId}-paragraph-${index + 1}`,
		content: line.length > 0 ? [schema.createText(line, { id: `${documentId}-text-${index + 1}` })] : [],
	}));
	return schema.createDocument(paragraphs, documentId);
}

/** Converts a supported text selection into clipboard-friendly plain text. */
export function documentSelectionToText(document: DocumentNode, selection: DocumentSelection): string | undefined {
	if (selection.kind === "all") return documentToPlainText(document);
	if (selection.kind !== "text") return undefined;
	const anchor = findTextBlockLocation(document, selection.anchor.nodeId);
	const head = findTextBlockLocation(document, selection.head.nodeId);
	if (!anchor || !head) return undefined;
	if (anchor.block.id === head.block.id) {
		const forward = anchor.index < head.index || (anchor.index === head.index && selection.anchor.offset <= selection.head.offset);
		const start = forward ? { location: anchor, point: selection.anchor } : { location: head, point: selection.head };
		const end = forward ? { location: head, point: selection.head } : { location: anchor, point: selection.anchor };
		return textFromBlockRange(start.location.block, start.location.index, start.point.offset, end.location.index, end.point.offset);
	}
	if (anchor.parent.id !== head.parent.id) return undefined;
	const forward = anchor.parentIndex < head.parentIndex;
	const start = forward ? { location: anchor, point: selection.anchor } : { location: head, point: selection.head };
	const end = forward ? { location: head, point: selection.head } : { location: anchor, point: selection.anchor };
	for (let index = start.location.parentIndex; index <= end.location.parentIndex; index += 1) {
		if (!isTextBlock(start.location.parent.content[index]!)) return undefined;
	}
	const parts: string[] = [textFromBlockRange(start.location.block, start.location.index, start.point.offset, start.location.block.content.length - 1, Number.MAX_SAFE_INTEGER)];
	for (let index = start.location.parentIndex + 1; index < end.location.parentIndex; index += 1) parts.push(textFromBlock(start.location.parent.content[index]!));
	parts.push(textFromBlockRange(end.location.block, 0, 0, end.location.index, end.point.offset));
	return parts.join("\n");
}

/** Converts the complete structured document to interoperable plain text. */
export function documentToPlainText(document: DocumentNode): string {
	const blocks: string[] = [];
	collectPlainTextBlocks(document, blocks);
	return blocks.join("\n");
}

function collectPlainTextBlocks(node: DocumentNode, blocks: string[]): void {
	if (isTextBlock(node)) {
		blocks.push(textFromBlock(node));
		return;
	}
	for (const child of node.content) collectPlainTextBlocks(child, blocks);
}

interface TextBlockLocation {
	readonly block: DocumentNode;
	readonly parent: DocumentNode;
	readonly parentIndex: number;
	readonly index: number;
}

function findTextBlockLocation(root: DocumentNode, textNodeId: DocumentNodeId): TextBlockLocation | undefined {
	const textLocation = findDocumentNode(root, textNodeId);
	const block = textLocation?.parent;
	if (!block || !isTextBlock(block)) return undefined;
	const blockLocation = findDocumentNode(root, block.id);
	const index = block.content.findIndex(child => child.id === textNodeId && child.text !== undefined);
	if (!blockLocation?.parent || index < 0) return undefined;
	return { block, parent: blockLocation.parent, parentIndex: blockLocation.index, index };
}

function textFromBlock(block: DocumentNode): string {
	return textFromBlockRange(block, 0, 0, block.content.length - 1, Number.MAX_SAFE_INTEGER);
}

function textFromBlockRange(block: DocumentNode, startIndex: number, startOffset: number, endIndex: number, endOffset: number): string {
	if (block.content.length === 0 || startIndex > endIndex) return "";
	const parts: string[] = [];
	for (let index = startIndex; index <= endIndex; index += 1) {
		const child = block.content[index]!;
		if (child.text !== undefined) {
			const from = index === startIndex ? Math.min(startOffset, child.text.length) : 0;
			const to = index === endIndex ? Math.min(endOffset, child.text.length) : child.text.length;
			if (to > from) parts.push(child.text.slice(from, to));
		} else if (child.type === "hardBreak") {
			parts.push("\n");
		} else if (child.type === "image") {
			parts.push(typeof child.attrs.alt === "string" ? child.attrs.alt : "\uFFFC");
		} else {
			parts.push(typeof child.attrs.label === "string" ? child.attrs.label : "\uFFFC");
		}
	}
	return parts.join("");
}

function isTextBlock(node: DocumentNode): boolean {
	return node.type === "paragraph" || node.type === "heading" || node.type === "textBlock";
}
