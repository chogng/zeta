import type { DocumentAttributes, DocumentNodeId } from './document.js';

/** One top-level TextModel Group with exactly one BlockTree. */
export interface TextModelGroup {
	readonly id: DocumentNodeId;
	readonly type: string;
	readonly attrs: DocumentAttributes;
	readonly startLine: number;
	readonly endLine: number;
	readonly blockTree: TextModelBlockTree;
}

/** One typed Block in a Group-owned BlockTree. */
export interface TextModelBlock {
	readonly id: DocumentNodeId;
	readonly type: string;
	readonly parentBlockId: DocumentNodeId | undefined;
	readonly startLine: number;
	readonly endLine: number;
	readonly attrs: DocumentAttributes;
	readonly children: readonly TextModelBlock[];
}

/** Immutable block topology and line ownership for one Group. */
export class TextModelBlockTree {
	public readonly roots: readonly TextModelBlock[];
	public readonly blocks: readonly TextModelBlock[];
	private readonly blocksById: ReadonlyMap<DocumentNodeId, TextModelBlock>;

	constructor(public readonly groupId: DocumentNodeId, roots: readonly TextModelBlock[]) {
		this.roots = Object.freeze([...roots]);
		validateSiblingRanges(this.roots, `Group '${groupId}'`);
		const blocks: TextModelBlock[] = [];
		const blocksById = new Map<DocumentNodeId, TextModelBlock>();
		for (const root of this.roots) this.indexBlock(root, undefined, blocks, blocksById);
		this.blocks = Object.freeze(blocks);
		this.blocksById = blocksById;
	}

	public getBlock(id: DocumentNodeId): TextModelBlock | undefined {
		return this.blocksById.get(id);
	}

	public getBlockAtLine(lineIndex: number): TextModelBlock | undefined {
		if (!Number.isSafeInteger(lineIndex) || lineIndex < 0) throw new RangeError('Block line index must be a non-negative safe integer');
		return findBlockAtLine(this.roots, lineIndex);
	}

	private indexBlock(
		block: TextModelBlock,
		parentBlockId: DocumentNodeId | undefined,
		blocks: TextModelBlock[],
		blocksById: Map<DocumentNodeId, TextModelBlock>,
	): void {
		if (block.parentBlockId !== parentBlockId) throw new Error(`Block '${block.id}' has an invalid parent`);
		if (blocksById.has(block.id)) throw new Error(`Duplicate block '${block.id}' in Group '${this.groupId}'`);
		if (!Number.isSafeInteger(block.startLine) || !Number.isSafeInteger(block.endLine) || block.startLine < 0 || block.endLine < block.startLine) {
			throw new RangeError(`Block '${block.id}' has an invalid TextBuffer line range`);
		}
		blocks.push(block);
		blocksById.set(block.id, block);
		validateSiblingRanges(block.children, `Block '${block.id}'`);
		for (const child of block.children) {
			if (child.startLine < block.startLine || child.endLine > block.endLine) throw new RangeError(`Child Block '${child.id}' falls outside Block '${block.id}'`);
			this.indexBlock(child, block.id, blocks, blocksById);
		}
	}
}

/** Creates the native one-Group, one-code-Block topology for a line-based model. */
export function createTextModelCodeGroup(lineCount: number): TextModelGroup {
	if (!Number.isSafeInteger(lineCount) || lineCount < 1) throw new RangeError('TextModel line count must be a positive safe integer');
	const block: TextModelBlock = Object.freeze({
		id: 'source:code',
		type: 'codeBlock',
		parentBlockId: undefined,
		startLine: 0,
		endLine: lineCount,
		attrs: EMPTY_ATTRIBUTES,
		children: Object.freeze([]),
	});
	return Object.freeze({
		id: 'source',
		type: 'source',
		attrs: EMPTY_ATTRIBUTES,
		startLine: 0,
		endLine: lineCount,
		blockTree: new TextModelBlockTree('source', [block]),
	});
}

const EMPTY_ATTRIBUTES: DocumentAttributes = Object.freeze({});

function findBlockAtLine(blocks: readonly TextModelBlock[], lineIndex: number): TextModelBlock | undefined {
	for (const block of blocks) {
		if (lineIndex < block.startLine || lineIndex >= block.endLine) continue;
		return findBlockAtLine(block.children, lineIndex) ?? block;
	}
	return undefined;
}

function validateSiblingRanges(blocks: readonly TextModelBlock[], owner: string): void {
	let previousEndLine = -1;
	for (const block of blocks) {
		if (block.startLine < previousEndLine) throw new RangeError(`${owner} contains overlapping Block ranges`);
		previousEndLine = block.endLine;
	}
}
