import { type TextModel } from '../model/textModel.js';

const DEFAULT_CURSOR_HISTORY_LIMIT = 100;
const DEFAULT_SELECTION_HISTORY_LIMIT = 1_000;

export class CursorControllerContext {
	public readonly selectionHistoryLimit: number;
	public readonly cursorHistoryLimit: number;
	public readonly readOnly: boolean;

	constructor(public readonly model: TextModel, options: { readonly selectionHistoryLimit?: number; readonly cursorHistoryLimit?: number; readonly readOnly?: boolean }) {
		this.selectionHistoryLimit = readLimit(options.selectionHistoryLimit, DEFAULT_SELECTION_HISTORY_LIMIT, 'selectionHistoryLimit');
		this.cursorHistoryLimit = readLimit(options.cursorHistoryLimit, DEFAULT_CURSOR_HISTORY_LIMIT, 'cursorHistoryLimit');
		if (options.readOnly !== undefined && typeof options.readOnly !== 'boolean') throw new TypeError('Editor read-only mode must be boolean');
		this.readOnly = options.readOnly ?? false;
	}
}

function readLimit(value: number | undefined, defaultValue: number, name: string): number {
	const limit = value ?? defaultValue;
	if (!Number.isSafeInteger(limit) || limit < 0) throw new RangeError(`${name} must be a non-negative safe integer`);
	return limit;
}
