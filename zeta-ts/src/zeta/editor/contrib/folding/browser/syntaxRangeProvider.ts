import { EditorFoldingRangeSource, type EditorFoldingRange } from "./foldingRanges.js";
import { type ILanguageConfigurationService } from '../../../common/languages/languageConfigurationRegistry.js';
import { assertLanguageId } from "../../../common/languages/languageId.js";
import { createLanguageLexicalLineScanner } from "../../../common/languages/languageLexicalConfiguration.js";
import { type LanguageLexicalState } from "../../../common/languages/languageLexicalLineScanner.js";
import { type TextModel } from "../../../common/model/textModel.js";

interface OpenBracketFold {
	readonly startLineIndex: number;
	readonly matchingToken: string;
}

interface OpenMarkerFold {
	readonly startLineIndex: number;
}

/**
 * Computes synchronous structural fold ranges from Stanza's configured lexical scanner.
 *
 * Brace, bracket, multi-line block-comment, and configured named region markers
 * participate. Parentheses are intentionally excluded: multi-line argument lists
 * remain editor text rather than becoming accidental fold headers. Callers may merge
 * these provider-owned ranges with indentation folds without exposing scanner state to
 * browser code.
 */
export function computeEditorLanguageFoldingRanges(model: TextModel, languageId: string, configurations: ILanguageConfigurationService): readonly EditorFoldingRange[] {
	assertLanguageId(languageId);
	if (!configurations || typeof configurations.getLanguageConfiguration !== "function") {
		throw new TypeError("Language folding requires language configurations");
	}
	const configuration = configurations.getLanguageConfiguration(languageId);
	const scanner = createLanguageLexicalLineScanner(languageId, configuration);
	const bracketStack: OpenBracketFold[] = [];
	const blockCommentStarts: number[] = [];
	const markerStarts: OpenMarkerFold[] = [];
	const ranges: EditorFoldingRange[] = [];
	let state: LanguageLexicalState = "normal";
	for (let lineIndex = 0; lineIndex < model.lineCount; lineIndex += 1) {
		const line = model.getLineContent((lineIndex) + 1);
		const result = scanner.scan(line, state);
		state = result.outputState;
		if (matchesMarker(configuration.foldingRules.markers?.end, line)) {
			const start = markerStarts.pop();
			if (start) appendRange(ranges, start.startLineIndex, lineIndex);
		} else if (matchesMarker(configuration.foldingRules.markers?.start, line)) {
			markerStarts.push(Object.freeze({ startLineIndex: lineIndex }));
		}
		for (const event of result.events) {
			if (event.kind === "multiline") {
				if (event.lexicalKind !== "blockComment") continue;
				if (event.action === "open") {
					blockCommentStarts.push(lineIndex);
				} else {
					const startLineIndex = blockCommentStarts.pop();
					if (startLineIndex !== undefined) appendRange(ranges, startLineIndex, lineIndex);
				}
				continue;
			}
			if (event.kind !== "bracket") continue;
			if (event.action === "open" && isFoldOpeningToken(event.token)) {
				bracketStack.push(Object.freeze({ startLineIndex: lineIndex, matchingToken: event.matchingToken }));
				continue;
			}
			if (event.action !== "close") continue;
			const opener = bracketStack.at(-1);
			if (!opener || opener.matchingToken !== event.token) continue;
			bracketStack.pop();
			appendRange(ranges, opener.startLineIndex, lineIndex);
		}
	}
	return Object.freeze(normalizeFoldingRanges(ranges));
}

function matchesMarker(pattern: RegExp | undefined, line: string): boolean {
	if (!pattern) return false;
	// Configuration patterns are frozen at registration time. Recreate a
	// stateless matcher so global/sticky contributions cannot mutate `lastIndex`.
	return new RegExp(pattern.source, pattern.flags.replace(/[gy]/gu, "")).test(line);
}

/** Merges independently-derived provider ranges while retaining only nested or disjoint spans. */
export function mergeEditorFoldingRanges(...sources: readonly (readonly EditorFoldingRange[])[]): readonly EditorFoldingRange[] {
	if (sources.some(source => !Array.isArray(source))) throw new TypeError("Folding range sources must be arrays");
	const ranges = sources.flat().map(range => Object.freeze({
		startLineIndex: range.startLineIndex,
		endLineIndex: range.endLineIndex,
		collapsed: false,
		source: EditorFoldingRangeSource.Provider,
	}));
	return Object.freeze(normalizeFoldingRanges(ranges));
}

function isFoldOpeningToken(token: string): boolean {
	return token === "{" || token === "[";
}

function appendRange(ranges: EditorFoldingRange[], startLineIndex: number, endLineIndex: number): void {
	if (endLineIndex <= startLineIndex) return;
	ranges.push(Object.freeze({ startLineIndex, endLineIndex, collapsed: false, source: EditorFoldingRangeSource.Provider }));
}

function normalizeFoldingRanges(ranges: readonly EditorFoldingRange[]): readonly EditorFoldingRange[] {
	const ordered = [...ranges]
		.filter(range => Number.isSafeInteger(range.startLineIndex) && Number.isSafeInteger(range.endLineIndex) && range.startLineIndex >= 0 && range.endLineIndex > range.startLineIndex)
		.sort((left, right) => left.startLineIndex - right.startLineIndex || right.endLineIndex - left.endLineIndex);
	const result: EditorFoldingRange[] = [];
	const active: EditorFoldingRange[] = [];
	for (const range of ordered) {
		while (active.length > 0 && active.at(-1)!.endLineIndex < range.startLineIndex) active.pop();
		const enclosing = active.at(-1);
		if (enclosing && range.startLineIndex === enclosing.startLineIndex) continue;
		if (enclosing && range.endLineIndex > enclosing.endLineIndex) continue;
		const previous = result.at(-1);
		if (previous && previous.startLineIndex === range.startLineIndex && previous.endLineIndex === range.endLineIndex) continue;
		const normalized = Object.freeze({ ...range, collapsed: false, source: EditorFoldingRangeSource.Provider });
		result.push(normalized);
		active.push(normalized);
	}
	return result;
}
