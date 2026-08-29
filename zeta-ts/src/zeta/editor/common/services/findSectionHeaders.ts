import { Position } from "../core/position.js";
import { Range } from "../core/range.js";
import { type LanguageFoldingMarkers } from "../languages/languageConfiguration.js";

/** Line-oriented model surface needed by section-header discovery. */
export interface SectionHeaderFinderTarget {
	readonly lineCount: number;
	getLineContent(lineNumber: number): string;
}

export interface FindSectionHeaderOptions {
	readonly foldingMarkers?: LanguageFoldingMarkers;
	readonly findRegionSectionHeaders: boolean;
	readonly findMarkSectionHeaders: boolean;
	readonly markSectionHeaderRegex: string;
}

export interface SectionHeader {
	readonly range: Range;
	readonly text: string;
	readonly hasSeparatorLine: boolean;
	/** MARK matches must be confirmed against language token context before presentation. */
	readonly shouldBeInComments: boolean;
}

const TRIM_DASHES = /^-+|-+$/gu;

/** Finds named region and MARK headers without depending on a browser editor. */
export function findSectionHeaders(model: SectionHeaderFinderTarget, options: FindSectionHeaderOptions): readonly SectionHeader[] {
	validateTarget(model);
	const headers = [
		...(options.findRegionSectionHeaders && options.foldingMarkers ? collectRegionHeaders(model, options.foldingMarkers) : []),
		...(options.findMarkSectionHeaders ? collectMarkHeaders(model, options.markSectionHeaderRegex) : []),
	];
	return Object.freeze(headers.sort((left, right) => Position.compare(left.range.getStartPosition(), right.range.getStartPosition())));
}

function collectRegionHeaders(model: SectionHeaderFinderTarget, markers: LanguageFoldingMarkers): SectionHeader[] {
	const headers: SectionHeader[] = [];
	for (let lineIndex = 0; lineIndex < model.lineCount; lineIndex += 1) {
		const line = model.getLineContent(lineIndex + 1);
		const match = stateless(markers.start).exec(line);
		if (!match) continue;
		const startColumn = match.index + match[0].length;
		const header = headerText(line.slice(startColumn));
		if (!header.text && !header.hasSeparatorLine) continue;
		headers.push(Object.freeze({
			range: Range.fromPositions(new Position((lineIndex) + 1, (startColumn) + 1), new Position((lineIndex) + 1, (line.length) + 1)),
			...header,
			shouldBeInComments: false,
		}));
	}
	return headers;
}

function collectMarkHeaders(model: SectionHeaderFinderTarget, source: string): SectionHeader[] {
	if (!source.trim()) return [];
	let regex: RegExp;
	try {
		regex = new RegExp(source, "gm");
	} catch (error) {
		throw new TypeError("Section header MARK pattern must be a valid regular expression", { cause: error });
	}
	const lines = Array.from({ length: model.lineCount }, (_, lineIndex) => model.getLineContent(lineIndex + 1));
	const lineStarts: number[] = [];
	let offset = 0;
	for (const line of lines) {
		lineStarts.push(offset);
		offset += line.length + 1;
	}
	const text = lines.join("\n");
	const headers: SectionHeader[] = [];
	let match: RegExpExecArray | null;
	while ((match = regex.exec(text)) !== null) {
		const start = positionAtOffset(lineStarts, lines, match.index);
		const end = positionAtOffset(lineStarts, lines, match.index + match[0].length);
		const label = match.groups?.label ?? "";
		const hasSeparatorLine = Boolean(match.groups?.separator);
		if ((label || hasSeparatorLine) && (headers.at(-1)?.range.getEndPosition().lineNumber ?? -1) < start.lineNumber) {
			headers.push(Object.freeze({
				range: Range.fromPositions(start, end),
				text: label,
				hasSeparatorLine,
				shouldBeInComments: true,
			}));
		}
		if (match[0].length === 0) regex.lastIndex += 1;
	}
	return headers;
}

function positionAtOffset(lineStarts: readonly number[], lines: readonly string[], offset: number): Position {
	let low = 0;
	let high = lineStarts.length - 1;
	while (low < high) {
		const middle = Math.ceil((low + high) / 2);
		if (lineStarts[middle]! <= offset) low = middle;
		else high = middle - 1;
	}
	return new Position((low) + 1, (Math.min(offset - lineStarts[low]!, lines[low]!.length)) + 1);
}

function headerText(value: string): { readonly text: string; readonly hasSeparatorLine: boolean } {
	const trimmed = value.trim();
	return Object.freeze({
		text: trimmed.replace(TRIM_DASHES, ""),
		hasSeparatorLine: trimmed.startsWith("-"),
	});
}

function stateless(pattern: RegExp): RegExp {
	return new RegExp(pattern.source, pattern.flags.replace(/[gy]/gu, ""));
}

function validateTarget(model: SectionHeaderFinderTarget): void {
	if (!model || !Number.isSafeInteger(model.lineCount) || model.lineCount < 1 || typeof model.getLineContent !== "function") {
		throw new TypeError("Section header discovery requires a non-empty line model");
	}
}
