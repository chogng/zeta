import { isNonNegativeSafeInteger } from "../../../base/common/numbers.js";
import type { DocumentNode, DocumentNodeId } from "../model/document.js";
import type { DocumentPoint } from "./documentSelection.js";
import type { DocumentSchema } from "../model/documentSchema.js";

export type DocumentPositionBias = "backward" | "forward";

export interface DocumentPositionPathEntry {
	readonly node: DocumentNode;
	readonly start: number;
	readonly index: number;
}

/** A resolved tree path for one absolute document position. */
export interface ResolvedDocumentPosition {
	readonly pos: number;
	readonly bias: DocumentPositionBias;
	readonly point: DocumentPoint | undefined;
	readonly path: readonly DocumentPositionPathEntry[];
	readonly depth: number;
}

/** Returns the ProseMirror-style node size used by Stanza's absolute positions. */
export function documentNodeSize(node: DocumentNode, schema: DocumentSchema): number {
	if (node.text !== undefined) return node.text.length;
	if (isDocumentLeaf(node, schema)) return 1;
	return 2 + documentContentSize(node, schema);
}

/** Returns the size occupied by a node's content, excluding its boundaries. */
export function documentContentSize(node: DocumentNode, schema: DocumentSchema): number {
	if (node.text !== undefined) return node.text.length;
	if (isDocumentLeaf(node, schema)) return 0;
	return node.content.reduce((size, child) => size + documentNodeSize(child, schema), 0);
}

/** Converts an identity-based text point into an absolute position in the root. */
export function documentPointToPosition(document: DocumentNode, schema: DocumentSchema, point: DocumentPoint): number {
	const position = findPointPosition(document, schema, point, -1);
	if (position === undefined) throw new RangeError(`Text point '${point.nodeId}' does not exist in the document`);
	return position;
}

/** Converts an absolute position into the nearest text point using the requested boundary bias. */
export function documentPositionToPoint(document: DocumentNode, schema: DocumentSchema, pos: number, bias: DocumentPositionBias = "forward"): DocumentPoint | undefined {
	validateDocumentPosition(document, schema, pos);
	const ranges = collectTextRanges(document, schema, -1, []);
	if (ranges.length === 0) return undefined;
	const containing = ranges.filter(range => range.start <= pos && pos <= range.end);
	if (containing.length > 0) {
		const range = bias === "forward" ? containing[containing.length - 1]! : containing[0]!;
		return { nodeId: range.node.id, offset: pos - range.start };
	}
	if (bias === "forward") {
		const next = ranges.find(range => range.start > pos);
		return next ? { nodeId: next.node.id, offset: 0 } : lastTextPoint(ranges);
	}
	let previous: Range | undefined;
	for (const range of ranges) {
		if (range.end >= pos) break;
		previous = range;
	}
	return previous ? { nodeId: previous.node.id, offset: previous.node.text!.length } : { nodeId: ranges[0]!.node.id, offset: 0 };
}

/** Resolves an absolute position to its immutable root-to-text path. */
export function resolveDocumentPosition(document: DocumentNode, schema: DocumentSchema, pos: number, bias: DocumentPositionBias = "forward"): ResolvedDocumentPosition {
	const point = documentPositionToPoint(document, schema, pos, bias);
	const path = point ? findPointPath(document, schema, point.nodeId, -1, -1) : [{ node: document, start: -1, index: -1 }];
	const normalizedPath = Object.freeze(path.map(entry => Object.freeze(entry)));
	return Object.freeze({ pos, bias, point, path: normalizedPath, depth: Math.max(0, normalizedPath.length - 1) });
}

interface Range {
	readonly node: DocumentNode;
	readonly start: number;
	readonly end: number;
}

function findPointPosition(node: DocumentNode, schema: DocumentSchema, point: DocumentPoint, start: number): number | undefined {
	if (node.id === point.nodeId) {
		if (node.text === undefined) throw new RangeError(`Document point '${point.nodeId}' must target a text node`);
		if (!isNonNegativeSafeInteger(point.offset) || point.offset > node.text.length) throw new RangeError(`Document point offset must be between 0 and ${node.text.length}`);
		return start + point.offset;
	}
	if (node.text !== undefined || isDocumentLeaf(node, schema)) return undefined;
	let childStart = start + 1;
	for (const child of node.content) {
		const position = findPointPosition(child, schema, point, childStart);
		if (position !== undefined) return position;
		childStart += documentNodeSize(child, schema);
	}
	return undefined;
}

function collectTextRanges(node: DocumentNode, schema: DocumentSchema, start: number, ranges: Range[]): Range[] {
	if (node.text !== undefined) {
		ranges.push({ node, start, end: start + node.text.length });
		return ranges;
	}
	if (isDocumentLeaf(node, schema)) return ranges;
	let childStart = start + 1;
	for (const child of node.content) {
		collectTextRanges(child, schema, childStart, ranges);
		childStart += documentNodeSize(child, schema);
	}
	return ranges;
}

function findPointPath(node: DocumentNode, schema: DocumentSchema, nodeId: DocumentNodeId, start: number, index: number): DocumentPositionPathEntry[] {
	const entry = { node, start, index };
	if (node.id === nodeId) return [entry];
	if (node.text !== undefined || isDocumentLeaf(node, schema)) return [];
	let childStart = start + 1;
	for (let childIndex = 0; childIndex < node.content.length; childIndex += 1) {
		const child = node.content[childIndex]!;
		const path = findPointPath(child, schema, nodeId, childStart, childIndex);
		if (path.length > 0) return [entry, ...path];
		childStart += documentNodeSize(child, schema);
	}
	return [];
}

function validateDocumentPosition(document: DocumentNode, schema: DocumentSchema, pos: number): void {
	const contentSize = documentContentSize(document, schema);
	if (!isNonNegativeSafeInteger(pos) || pos > contentSize) throw new RangeError(`Document position must be between 0 and ${contentSize}`);
}

function lastTextPoint(ranges: readonly Range[]): DocumentPoint {
	const range = ranges[ranges.length - 1]!;
	return { nodeId: range.node.id, offset: range.node.text!.length };
}

function isDocumentLeaf(node: DocumentNode, schema: DocumentSchema): boolean {
	return schema.isLeafNode(node);
}
