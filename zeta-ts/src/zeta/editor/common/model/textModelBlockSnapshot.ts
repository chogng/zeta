import type { DocumentAttributes, DocumentNode, DocumentNodeId } from "./document.js";
import type { DocumentSchema } from "./documentSchema.js";
import { TextModelBlockTree, type TextModelBlock, type TextModelGroup } from "./textModelBlockTree.js";

/** Group and BlockTree metadata aligned to one TextBuffer snapshot. */
export class TextModelBlockSnapshot {
	readonly groups: readonly TextModelGroup[];
	private readonly text: string;

	constructor(groups: readonly TextModelGroup[], lines: readonly string[]) {
		this.groups = Object.freeze([...groups]);
		this.text = lines.join("\n");
	}

	getText(): string {
		return this.text;
	}
}

/** Builds Group and BlockTree metadata while legacy schema nodes migrate to native line ranges. */
export function createTextModelBlockSnapshot(schema: DocumentSchema, document: DocumentNode): TextModelBlockSnapshot {
	const pendingGroups: PendingGroup[] = [];
	const lines: string[] = [];
	let syntheticRoots: TextModelBlock[] = [];

	const appendLine = (text: string): void => {
		for (const content of text.split("\n")) lines.push(content);
	};

	const createBlock = (node: DocumentNode, parentBlockId: DocumentNodeId | undefined): TextModelBlock => {
		const startLine = lines.length;
		const children: TextModelBlock[] = [];
		const childLines = node.content.filter(child => schema.getNodeSpec(child.type)?.kind === "line");
		const hasAtomicPayload = typeof node.attrs.src === "string" || node.type === "imageBlock" || node.type === "horizontalRule";
		if (hasAtomicPayload) appendLine("\uFFFC");
		if (childLines.length > 0) {
			for (const child of childLines) appendLine(inlineText(child));
		} else if (hasDirectInlineContent(node) || isLegacyTextBlock(node)) {
			appendLine(inlineText(node));
		} else {
			for (const child of node.content) {
				const kind = schema.getNodeSpec(child.type)?.kind;
				if (kind === "group") throw new TypeError(`Group '${child.id}' cannot be nested in Block '${node.id}'`);
				if (kind === "block") children.push(createBlock(child, node.id));
			}
			if (lines.length === startLine) appendLine("");
		}
		return Object.freeze({
			id: node.id,
			type: node.type,
			parentBlockId,
			startLine,
			endLine: lines.length,
			attrs: node.attrs,
			children: Object.freeze(children),
		});
	};

	const createGroup = (node: DocumentNode): void => {
		const startLine = lines.length;
		const roots: TextModelBlock[] = [];
		for (const child of node.content) {
			const kind = schema.getNodeSpec(child.type)?.kind;
			if (kind === "group") throw new TypeError(`Group '${child.id}' cannot be nested in Group '${node.id}'`);
			if (kind === "block") roots.push(createBlock(child, undefined));
		}
		pendingGroups.push({ id: node.id, type: node.type, attrs: node.attrs, startLine, endLine: lines.length, roots });
	};

	const flushSyntheticGroup = (): void => {
		if (syntheticRoots.length === 0) return;
		const startLine = syntheticRoots[0]!.startLine;
		const endLine = syntheticRoots.at(-1)!.endLine;
		pendingGroups.push({ type: "group", attrs: EMPTY_ATTRIBUTES, startLine, endLine, roots: syntheticRoots });
		syntheticRoots = [];
	};

	const visitRootChild = (node: DocumentNode): void => {
		const kind = schema.getNodeSpec(node.type)?.kind;
		if (kind === "group") {
			flushSyntheticGroup();
			createGroup(node);
			return;
		}
		if (kind === "block") {
			syntheticRoots.push(createBlock(node, undefined));
			return;
		}
		for (const child of node.content) visitRootChild(child);
	};

	for (const child of document.content) visitRootChild(child);
	flushSyntheticGroup();
	if (pendingGroups.length === 0) pendingGroups.push({ type: "group", attrs: EMPTY_ATTRIBUTES, startLine: 0, endLine: 0, roots: [] });
	const syntheticGroupCount = pendingGroups.filter(group => group.id === undefined).length;
	let syntheticGroupIndex = 0;
	const groups = pendingGroups.map(group => {
		const id = group.id ?? (syntheticGroupCount === 1 ? `${document.id}:group` : `${document.id}:group:${syntheticGroupIndex++}`);
		return Object.freeze({
			id,
			type: group.type,
			attrs: group.attrs,
			startLine: group.startLine,
			endLine: group.endLine,
			blockTree: new TextModelBlockTree(id, group.roots),
		});
	});
	return new TextModelBlockSnapshot(groups, lines);
}

interface PendingGroup {
	readonly id?: DocumentNodeId;
	readonly type: string;
	readonly attrs: DocumentAttributes;
	readonly startLine: number;
	readonly endLine: number;
	readonly roots: readonly TextModelBlock[];
}

const EMPTY_ATTRIBUTES: DocumentAttributes = Object.freeze({});

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
