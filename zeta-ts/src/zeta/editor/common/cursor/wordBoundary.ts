import { TextPosition, TextRange } from "../core/text.js";
import { type TextModel } from "../model/textModel.js";
import { getTextWordSegments } from "../core/textSegmentation.js";

/**
 * Returns the complete text segment selected by a word-selection gesture.
 *
 * Word-like text, whitespace, and punctuation are all selectable segments.
 * The range never crosses a line and its UTF-16 boundaries never split a
 * Unicode code point. At end of line, the preceding segment is selected.
 */
export function getWordSelectionRange(model: TextModel, position: TextPosition, wordPattern?: RegExp): TextRange {
	model.offsetAt(position);
	const line = model.getLineContent(position.lineIndex);
	if (line.length === 0) return TextRange.emptyAt(position);
	const probe = position.columnIndex === line.length
		? line.length - 1
		: position.columnIndex;
	const patternRange = wordPatternRange(line, probe, wordPattern);
	if (patternRange) return TextRange.from(
		TextPosition.at(position.lineIndex, patternRange.start),
		TextPosition.at(position.lineIndex, patternRange.end),
	);
	const segment = getTextWordSegments(line).find(candidate =>
		probe >= candidate.start && probe < candidate.end
	);
	if (!segment) {
		throw new RangeError("Word-selection probe is outside the line");
	}
	return TextRange.from(
		TextPosition.at(position.lineIndex, segment.start),
		TextPosition.at(position.lineIndex, segment.end),
	);
}

/** Returns language-pattern words, or generic word-like segments when no pattern is configured. */
export function getTextWordRanges(text: string, wordPattern?: RegExp): readonly { readonly start: number; readonly end: number }[] {
	if (wordPattern) return Object.freeze(wordPatternRanges(text, wordPattern));
	return Object.freeze(getTextWordSegments(text).flatMap(segment => segment.wordLike ? [{ start: segment.start, end: segment.end }] : []));
}

function wordPatternRange(line: string, probe: number, pattern: RegExp | undefined): { readonly start: number; readonly end: number } | undefined {
	return pattern ? wordPatternRanges(line, pattern).find(range => probe >= range.start && probe < range.end) : undefined;
}

function wordPatternRanges(line: string, pattern: RegExp): readonly { readonly start: number; readonly end: number }[] {
	const flags = pattern.flags.replaceAll("y", "").includes("g")
		? pattern.flags.replaceAll("y", "")
		: `${pattern.flags.replaceAll("y", "")}g`;
	const matcher = new RegExp(pattern.source, flags);
	const ranges: { start: number; end: number }[] = [];
	for (let match = matcher.exec(line); match; match = matcher.exec(line)) {
		if (match[0].length === 0) {
			matcher.lastIndex += 1;
			continue;
		}
		const start = match.index;
		const end = start + match[0].length;
		ranges.push({ start, end });
	}
	return ranges;
}
