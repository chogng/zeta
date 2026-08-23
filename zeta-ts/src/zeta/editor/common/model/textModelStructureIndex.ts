import type { DocumentAttributes, DocumentNode, DocumentNodeId } from "./document.js";
import type { DocumentSchema } from "./documentSchema.js";

/** One durable Group projected over ordered TextModel blocks. */
export interface TextModelGroup {
	readonly id: DocumentNodeId;
	readonly type: string;
	readonly blockIds: readonly DocumentNodeId[];
}

/** One typed Block anchored to an end-exclusive TextModel line range. */
export interface TextModelBlock {
	readonly id: DocumentNodeId;
	readonly type: string;
	readonly groupId: DocumentNodeId;
	readonly startLine: number;
	readonly endLine: number;
	readonly attrs: DocumentAttributes;
}

/** One model line owned by a typed Block. */
export interface TextModelStructuredLine {
	readonly id: DocumentNodeId;
	readonly type: string;
	readonly blockId: DocumentNodeId;
	readonly lineIndex: number;
	readonly text: string;
}

/** Immutable Group > Block > Line index over one TextModel snapshot. */
export class TextModelStructureIndex {
	readonly groups: readonly TextModelGroup[];
	readonly blocks: readonly TextModelBlock[];
	readonly lines: readonly TextModelStructuredLine[];
	private readonly groupsById: ReadonlyMap<DocumentNodeId, TextModelGroup>;
	private readonly blocksById: ReadonlyMap<DocumentNodeId, TextModelBlock>;

	constructor(groups: readonly TextModelGroup[], blocks: readonly TextModelBlock[], lines: readonly TextModelStructuredLine[]) {
		this.groups = Object.freeze([...groups]);
		this.blocks = Object.freeze([...blocks]);
		this.lines = Object.freeze([...lines]);
		this.groupsById = new Map(this.groups.map(group => [group.id, group]));
		this.blocksById = new Map(this.blocks.map(block => [block.id, block]));
	}

	getGroup(id: DocumentNodeId): TextModelGroup | undefined {
		return this.groupsById.get(id);
	}

	getBlock(id: DocumentNodeId): TextModelBlock | undefined {
		return this.blocksById.get(id);
	}

	getBlockAtLine(lineIndex: number): TextModelBlock | undefined {
		if (!Number.isSafeInteger(lineIndex) || lineIndex < 0 || lineIndex >= this.lines.length) throw new RangeError("Structured line index is outside the TextModel");
		const blockId = this.lines[lineIndex]?.blockId;
		return blockId ? this.blocksById.get(blockId) : undefined;
	}

	getText(): string {
		return this.lines.map(line => line.text).join("\n");
	}
}

/** Builds the compatibility index while legacy document nodes migrate to native line ranges. */
export function createTextModelStructureIndex(schema: DocumentSchema, document: DocumentNode): TextModelStructureIndex {
	const groups: TextModelGroup[] = [];
	const blocks: TextModelBlock[] = [];
	const lines: TextModelStructuredLine[] = [];
	const syntheticRootGroupId = `${document.id}:group`;
	const syntheticBlockIds: DocumentNodeId[] = [];

	const appendLine = (node: DocumentNode, blockId: DocumentNodeId, type: string, text: string, part = 0): void => {
		for (const [offset, content] of text.split("\n").entries()) {
			lines.push(Object.freeze({
				id: part === 0 && offset === 0 ? node.id : `${node.id}:line:${part + offset}`,
				type,
				blockId,
				lineIndex: lines.length,
				text: content,
			}));
		}
	};

	const visitBlock = (node: DocumentNode, groupId: DocumentNodeId): void => {
		const startLine = lines.length;
		const childLines = node.content.filter(child => schema.getNodeSpec(child.type)?.kind === "line");
		const hasAtomicPayload = typeof node.attrs.src === "string" || node.type === "imageBlock" || node.type === "horizontalRule";
		if (hasAtomicPayload) appendLine(node, node.id, "objectLine", "\uFFFC");
		if (childLines.length > 0) {
			for (const child of childLines) appendLine(child, node.id, child.type, inlineText(child));
		} else if (hasDirectInlineContent(node) || isLegacyTextBlock(node)) {
			appendLine(node, node.id, "textLine", inlineText(node));
		} else {
			for (const child of node.content) visit(child, groupId);
			if (lines.length === startLine) appendLine(node, node.id, "emptyLine", "");
		}
		blocks.push(Object.freeze({ id: node.id, type: node.type, groupId, startLine, endLine: lines.length, attrs: node.attrs }));
	};

	const visitGroup = (node: DocumentNode): void => {
		const blockIds: DocumentNodeId[] = [];
		const groupIndex = groups.length;
		groups.push(Object.freeze({ id: node.id, type: node.type, blockIds }));
		for (const child of node.content) {
			if (schema.getNodeSpec(child.type)?.kind === "block") blockIds.push(child.id);
			visit(child, node.id);
		}
		const group = groups[groupIndex]!;
		groups[groupIndex] = Object.freeze({ ...group, blockIds: Object.freeze([...blockIds]) });
	};

	const visit = (node: DocumentNode, groupId: DocumentNodeId): void => {
		const kind = schema.getNodeSpec(node.type)?.kind;
		if (kind === "group") {
			visitGroup(node);
			return;
		}
		if (kind === "block") {
			if (groupId === syntheticRootGroupId) syntheticBlockIds.push(node.id);
			visitBlock(node, groupId);
			return;
		}
		for (const child of node.content) visit(child, groupId);
	};

	for (const child of document.content) visit(child, syntheticRootGroupId);
	if (syntheticBlockIds.length > 0) groups.unshift(Object.freeze({ id: syntheticRootGroupId, type: "group", blockIds: Object.freeze([...syntheticBlockIds]) }));
	return new TextModelStructureIndex(groups, blocks, lines);
}

function hasDirectInlineContent(node: DocumentNode): boolean {
	return node.content.some(child => child.text !== undefined || child.type === "hardBreak" || child.type === "image");
}

function isLegacyTextBlock(node: DocumentNode): boolean {
	return node.type === "paragraph" || node.type === "heading" || node.type === "codeBlock";
}

function inlineText(node: DocumentNode): string {
	return node.content.map(child => {
		if (child.text !== undefined) return child.text;
		if (child.type === "hardBreak") return "\n";
		if (child.type === "image") return typeof child.attrs.alt === "string" ? child.attrs.alt : "\uFFFC";
		return typeof child.attrs.label === "string" ? child.attrs.label : "\uFFFC";
	}).join("");
}
