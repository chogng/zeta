import { EditorCommandHistoryMode, type EditorEditCommand } from "../../../common/commands/editorEditCommand.js";
import { normalizeTextLineEndings } from "../../../common/core/textChange.js";

import { type TextModel } from "../../../common/model/textModel.js";
import { type TextSearchMatch } from "../../../common/model/textModelSearch.js";
import { type TextEdit } from '../../../common/languages.js';

/** Expands JavaScript-style regular-expression replacement tokens against one search match. */
export function resolveTextSearchReplacement(match: TextSearchMatch, replacement: string): string {
	if (typeof replacement !== "string") throw new TypeError("Text search replacement must be a string");
	return replacement.replace(/\$(\$|&|\d{1,2}|<[^>]*>)/g, (token, reference: string) => {
		if (reference === "$") return "$";
		if (reference === "&") return match.text;
		if (reference.startsWith("<")) {
			const name = reference.slice(1, -1);
			return Object.hasOwn(match.namedCaptures, name) ? match.namedCaptures[name] ?? "" : token;
		}
		const captureIndex = Number(reference);
		if (captureIndex === 0 || captureIndex > match.captures.length) return token;
		return match.captures[captureIndex - 1] ?? "";
	});
}

/** Replaces one current-version search match as an isolated undo step. */
export function createReplaceTextMatchCommand(model: TextModel, match: TextSearchMatch, replacement: string): EditorEditCommand {
	assertCurrentMatch(model, match);
	const normalized = normalizeTextLineEndings(replacement);
	const startOffset = model.offsetAt(match.range.getStartPosition());
	return Object.freeze({
		edits: Object.freeze([{ range: match.range, text: normalized }]),
		selectionsAfter: Object.freeze([Object.freeze({
			anchorOffset: startOffset + normalized.length,
			activeOffset: startOffset + normalized.length,
		})]),
		primarySelectionIndex: 0,
		historyMode: EditorCommandHistoryMode.Isolated,
	});
}

/** Replaces ordered, non-overlapping current-version matches as one isolated undo step. */
export function createReplaceAllTextMatchesCommand(model: TextModel, matches: readonly TextSearchMatch[], replacements: readonly string[]): EditorEditCommand {
	if (matches.length !== replacements.length) {
		throw new RangeError("Text search replacements must match the result count");
	}
	if (matches.length === 0) throw new RangeError("Replace all requires at least one text search match");

	const edits: TextEdit[] = [];
	let cumulativeDelta = 0;
	let caretOffset = 0;
	let previousEndOffset = -1;
	for (let index = 0; index < matches.length; index += 1) {
		const match = matches[index]!;
		const replacement = normalizeTextLineEndings(replacements[index]!);
		assertCurrentMatch(model, match);
		const startOffset = model.offsetAt(match.range.getStartPosition());
		const endOffset = model.offsetAt(match.range.getEndPosition());
		if (startOffset < previousEndOffset) throw new RangeError("Text search matches must not overlap");
		edits.push(Object.freeze({ range: match.range, text: replacement }));
		caretOffset = startOffset + cumulativeDelta + replacement.length;
		cumulativeDelta += replacement.length - (endOffset - startOffset);
		previousEndOffset = endOffset;
	}

	return Object.freeze({
		edits: Object.freeze(edits),
		selectionsAfter: Object.freeze([Object.freeze({
			anchorOffset: caretOffset,
			activeOffset: caretOffset,
		})]),
		primarySelectionIndex: 0,
		historyMode: EditorCommandHistoryMode.Isolated,
	});
}

function assertCurrentMatch(model: TextModel, match: TextSearchMatch): void {
	if (match.modelVersion !== model.version) throw new Error("Text search match belongs to a stale model version");
	if (model.getTextInRange(match.range) !== match.text) throw new Error("Text search match no longer identifies the same text");
}
