import { EditorCommandHistoryMode, type EditorEditCommand, type TextSelectionOffsets } from "../../../common/commands/editorEditCommand.js";
import { type Selection } from "../../../common/core/selection.js";
import { Position } from "../../../common/core/position.js";
import { Range } from "../../../common/core/range.js";

import { type TextModel } from "../../../common/model/textModel.js";
import { type TextEdit } from '../../../common/languages.js';

export interface EditorBlockCommentOptions {
	readonly open: string;
	readonly close: string;
	readonly insertSpace?: boolean;
}

interface OffsetEdit {
	readonly startOffset: number;
	readonly endOffset: number;
	readonly text: string;
	readonly edit: TextEdit;
}

type BlockCommentPlan = AddRangePlan | AddCaretPlan | RemoveRangePlan;

interface BasePlan {
	readonly anchorOffset: number;
	readonly activeOffset: number;
	readonly startOffset: number;
	readonly endOffset: number;
	readonly edits: readonly OffsetEdit[];
}

interface AddRangePlan extends BasePlan {
	readonly kind: "addRange";
}

interface AddCaretPlan extends BasePlan {
	readonly kind: "addCaret";
	readonly caretOffsetInInsertedText: number;
}

interface RemoveRangePlan extends BasePlan {
	readonly kind: "removeRange";
}

/** Toggles one configured block-comment pair around each non-overlapping selection. */
export function createToggleBlockCommentCommand(model: TextModel, selections: readonly Selection[], options: EditorBlockCommentOptions): EditorEditCommand {
	const tokens = readTokens(options);
	const snapshot = model.createVersionedSnapshot();
	const text = snapshot.getText();
	const plans = selections.map(selection => createPlan(
		model,
		text,
		model.offsetAt(selection.getSelectionStart()),
		model.offsetAt(selection.getPosition()),
		tokens,
	));
	validatePlans(plans);
	const edits = Object.freeze(plans.flatMap(plan => plan.edits).sort((left, right) => left.startOffset - right.startOffset));
	const selectionsAfter = Object.freeze(plans.map(plan => selectionAfterPlan(plan, edits)));
	return Object.freeze({
		edits: Object.freeze(edits.map(edit => edit.edit)),
		selectionsAfter,
		primarySelectionIndex: 0,
		historyMode: EditorCommandHistoryMode.Isolated,
	});
}

function createPlan(model: TextModel, text: string, anchorOffset: number, activeOffset: number, tokens: Required<EditorBlockCommentOptions>): BlockCommentPlan {
	const startOffset = Math.min(anchorOffset, activeOffset);
	const endOffset = Math.max(anchorOffset, activeOffset);
	if (startOffset === endOffset) {
		const inserted = `${tokens.open} ${tokens.close}`;
		return Object.freeze({
			kind: "addCaret",
			anchorOffset,
			activeOffset,
			startOffset,
			endOffset,
			caretOffsetInInsertedText: tokens.open.length + 1,
			edits: Object.freeze([offsetEdit(model, startOffset, startOffset, inserted)]),
		});
	}
	const wrapped = surroundingTokens(text, startOffset, endOffset, tokens);
	if (wrapped) {
		return Object.freeze({
			kind: "removeRange",
			anchorOffset,
			activeOffset,
			startOffset,
			endOffset,
			edits: Object.freeze([
				offsetEdit(model, wrapped.openStartOffset, startOffset, ""),
				offsetEdit(model, endOffset, wrapped.closeEndOffset, ""),
			]),
		});
	}
	const prefix = tokens.open + (tokens.insertSpace ? " " : "");
	const suffix = (tokens.insertSpace ? " " : "") + tokens.close;
	return Object.freeze({
		kind: "addRange",
		anchorOffset,
		activeOffset,
		startOffset,
		endOffset,
		edits: Object.freeze([
			offsetEdit(model, startOffset, startOffset, prefix),
			offsetEdit(model, endOffset, endOffset, suffix),
		]),
	});
}

function surroundingTokens(text: string, startOffset: number, endOffset: number, tokens: Required<EditorBlockCommentOptions>): { readonly openStartOffset: number; readonly closeEndOffset: number } | undefined {
	const openWithOptionalSpace = text.slice(startOffset - tokens.open.length - 1, startOffset);
	const openStartOffset = openWithOptionalSpace === `${tokens.open} `
		? startOffset - tokens.open.length - 1
		: text.slice(startOffset - tokens.open.length, startOffset) === tokens.open
			? startOffset - tokens.open.length
			: undefined;
	if (openStartOffset === undefined) return undefined;
	const closeWithOptionalSpace = text.slice(endOffset, endOffset + tokens.close.length + 1);
	const closeEndOffset = closeWithOptionalSpace === ` ${tokens.close}`
		? endOffset + tokens.close.length + 1
		: text.slice(endOffset, endOffset + tokens.close.length) === tokens.close
			? endOffset + tokens.close.length
			: undefined;
	return closeEndOffset === undefined
		? undefined
		: Object.freeze({ openStartOffset, closeEndOffset });
}

function validatePlans(plans: readonly BlockCommentPlan[]): void {
	const ranges = plans.map(plan => Object.freeze({ startOffset: plan.startOffset, endOffset: plan.endOffset })).sort((left, right) => left.startOffset - right.startOffset || left.endOffset - right.endOffset);
	for (let index = 1; index < ranges.length; index += 1) {
		const previous = ranges[index - 1]!;
		const current = ranges[index]!;
		if (current.startOffset <= previous.endOffset) {
			throw new RangeError("Block comment selections must not overlap or touch");
		}
	}
}

function selectionAfterPlan(plan: BlockCommentPlan, edits: readonly OffsetEdit[]): TextSelectionOffsets {
	if (plan.kind === "addCaret") {
		const offset = mapOffsetThroughEdits(plan.startOffset, edits, "before") + plan.caretOffsetInInsertedText;
		return Object.freeze({ anchorOffset: offset, activeOffset: offset });
	}
	const startOffset = mapOffsetThroughEdits(plan.startOffset, edits, plan.kind === "addRange" ? "after" : "after");
	const endOffset = mapOffsetThroughEdits(plan.endOffset, edits, plan.kind === "addRange" ? "before" : "before");
	return plan.anchorOffset <= plan.activeOffset
		? Object.freeze({ anchorOffset: startOffset, activeOffset: endOffset })
		: Object.freeze({ anchorOffset: endOffset, activeOffset: startOffset });
}

function offsetEdit(model: TextModel, startOffset: number, endOffset: number, text: string): OffsetEdit {
	const start = model.positionAt(startOffset);
	const end = model.positionAt(endOffset);
	return Object.freeze({
		startOffset,
		endOffset,
		text,
		edit: Object.freeze({ range: Range.fromPositions(start, end), text }),
	});
}

function mapOffsetThroughEdits(offset: number, edits: readonly OffsetEdit[], insertionAffinity: "before" | "after"): number {
	let delta = 0;
	for (const edit of edits) {
		if (offset < edit.startOffset) break;
		if (edit.startOffset === edit.endOffset) {
			if (offset === edit.startOffset && insertionAffinity === "before") continue;
			delta += edit.text.length;
			continue;
		}
		if (offset <= edit.endOffset) {
			return edit.startOffset + delta + Math.min(offset - edit.startOffset, edit.text.length);
		}
		delta += edit.text.length - (edit.endOffset - edit.startOffset);
	}
	return offset + delta;
}

function readTokens(options: EditorBlockCommentOptions): Required<EditorBlockCommentOptions> {
	if (!options || typeof options !== "object" || typeof options.open !== "string" || typeof options.close !== "string") {
		throw new TypeError("Block comment command requires open and close tokens");
	}
	if (options.open.length === 0 || options.close.length === 0 || /[\r\n]/.test(options.open) || /[\r\n]/.test(options.close)) {
		throw new RangeError("Block comment tokens must be non-empty single-line strings");
	}
	if (options.insertSpace !== undefined && typeof options.insertSpace !== "boolean") {
		throw new TypeError("Block comment insertSpace must be a boolean");
	}
	return Object.freeze({ open: options.open, close: options.close, insertSpace: options.insertSpace ?? true });
}
