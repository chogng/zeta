import { type URI } from '../../../../base/common/uri.js';
import { type EditorInput } from '../../../browser/parts/editor/editorInput.js';
import { EditorPaneMatch } from '../../../browser/parts/editor/editorPane.js';

export const MULTI_DIFF_EDITOR_ID = 'stanza.editor.multiDiff';
export const MULTI_DIFF_EDITOR_CONTENT_TYPE = 'application/vnd.stanza.editor-multi-diff';

export interface MultiDiffEditorInputItem {
	readonly label: string;
	readonly original: EditorInput;
	readonly modified: EditorInput;
}

export interface MultiDiffEditorInput extends EditorInput {
	readonly contentType: typeof MULTI_DIFF_EDITOR_CONTENT_TYPE;
	readonly items: readonly MultiDiffEditorInputItem[];
}

/** Creates one Workbench tab containing an ordered collection of text comparisons. */
export function createMultiDiffEditorInput(resource: URI, items: readonly MultiDiffEditorInputItem[], label: string): MultiDiffEditorInput {
	if (!resource || typeof resource.toString !== 'function') throw new TypeError('Multi-diff editor input requires a resource identity');
	if (!Array.isArray(items) || items.length === 0) throw new TypeError('Multi-diff editor input requires at least one comparison');
	if (typeof label !== 'string' || label.trim().length === 0) throw new TypeError('Multi-diff editor label must be a non-empty string');
	const keys = new Set<string>();
	const normalizedItems = items.map((item) => {
		if (!item || typeof item !== 'object' || typeof item.label !== 'string' || item.label.trim().length === 0) {
			throw new TypeError('Multi-diff editor items require a non-empty label');
		}
		assertTextResourceInput(item.original, 'Multi-diff original input');
		assertTextResourceInput(item.modified, 'Multi-diff modified input');
		const normalized = Object.freeze({
			label: item.label.trim(),
			original: item.original,
			modified: item.modified,
		});
		const key = multiDiffEditorItemKey(normalized);
		if (keys.has(key)) throw new TypeError(`Duplicate multi-diff comparison '${normalized.label}'`);
		keys.add(key);
		return normalized;
	});
	return Object.freeze({
		resource,
		contentType: MULTI_DIFF_EDITOR_CONTENT_TYPE,
		items: Object.freeze(normalizedItems),
		label: label.trim(),
		readOnly: true,
	});
}

export function isMultiDiffEditorInput(input: EditorInput): input is MultiDiffEditorInput {
	return input.contentType === MULTI_DIFF_EDITOR_CONTENT_TYPE &&
		'items' in input &&
		Array.isArray(input.items) &&
		input.items.length > 0 &&
		input.items.every((item) => isMultiDiffEditorInputItem(item));
}

export function matchMultiDiffEditor(input: EditorInput): EditorPaneMatch {
	return isMultiDiffEditorInput(input) ? EditorPaneMatch.Default : EditorPaneMatch.None;
}

export function multiDiffEditorItemKey(item: MultiDiffEditorInputItem): string {
	return `${item.original.resource.toString()}\0${item.modified.resource.toString()}`;
}

function isMultiDiffEditorInputItem(value: unknown): value is MultiDiffEditorInputItem {
	return typeof value === 'object' && value !== null &&
		'label' in value && typeof value.label === 'string' && value.label.trim().length > 0 &&
		'original' in value && isTextResourceInput(value.original) &&
		'modified' in value && isTextResourceInput(value.modified);
}

function assertTextResourceInput(value: unknown, owner: string): asserts value is EditorInput {
	if (!isTextResourceInput(value)) throw new TypeError(`${owner} requires an editor resource`);
}

function isTextResourceInput(value: unknown): value is EditorInput {
	return typeof value === 'object' && value !== null &&
		'resource' in value &&
		typeof (value as EditorInput).resource?.toString === 'function';
}
