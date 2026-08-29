import { TextSelection } from "../../../common/core/selection.js";
import { TextPosition, TextRange } from "../../../common/core/text.js";
import { type TextModel } from "../../../common/model/textModel.js";
import { WordOperations } from "../../../common/cursor/cursorWordOperations.js";

/** Expands one selection through word, parser, enclosing-pair, line, and document scopes. */
export function expandSmartSelection(model: TextModel, selection: TextSelection, wordPattern?: RegExp, syntaxRanges: readonly TextRange[] = []): TextSelection {
	const current = selection.range;
	const next = expandRange(model, current, wordPattern, syntaxRanges);
	return TextSelection.fromRange(next, selection.direction);
}

function expandRange(model: TextModel, range: TextRange, wordPattern: RegExp | undefined, syntaxRanges: readonly TextRange[]): TextRange {
	if (range.empty) return WordOperations.getWordSelectionRange(model, range.start, wordPattern);
	const structural = smallestStrictlyContainingRange(model, range, syntaxRanges);
	if (structural) return structural;
	const enclosing = findEnclosingPair(model, range);
	if (enclosing && !enclosing.equals(range)) return enclosing;
	const lineStart = TextPosition.at(range.start.lineIndex, 0);
	const lineEnd = TextPosition.at(range.end.lineIndex, model.getLineContent(range.end.lineIndex).length);
	const lineRange = TextRange.from(lineStart, lineEnd);
	if (!lineRange.equals(range)) return lineRange;
	return TextRange.from(TextPosition.at(0, 0), TextPosition.at(model.lineCount - 1, model.getLineContent(model.lineCount - 1).length));
}

function smallestStrictlyContainingRange(model: TextModel, current: TextRange, ranges: readonly TextRange[]): TextRange | undefined {
	let best: TextRange | undefined;
	let bestLength = Number.POSITIVE_INFINITY;
	for (const candidate of ranges) {
		if (candidate.equals(current) || !candidate.containsRange(current)) continue;
		const length = model.offsetAt(candidate.end) - model.offsetAt(candidate.start);
		if (length < bestLength) {
			best = candidate;
			bestLength = length;
		}
	}
	return best;
}

function findEnclosingPair(model: TextModel, range: TextRange): TextRange | undefined {
	const pairs: readonly [string, string][] = [["(", ")"], ["[", "]"], ["{", "}"], ["\"", "\""], ["'", "'"]];
	const startOffset = model.offsetAt(range.start);
	const endOffset = model.offsetAt(range.end);
	const text = model.getText();
	let best: TextRange | undefined;
	for (const [open, close] of pairs) {
		let openOffset = text.lastIndexOf(open, Math.max(0, startOffset - 1));
		while (openOffset >= 0) {
			const closeOffset = text.indexOf(close, Math.max(openOffset + open.length, endOffset));
			if (closeOffset < 0) break;
			const candidate = TextRange.from(model.positionAt(openOffset), model.positionAt(closeOffset + close.length));
			if (candidate.containsRange(range) && (!best || candidate.length.lineCount < best.length.lineCount || candidate.length.columnCount < best.length.columnCount)) best = candidate;
			openOffset = text.lastIndexOf(open, openOffset - 1);
		}
	}
	return best;
}
