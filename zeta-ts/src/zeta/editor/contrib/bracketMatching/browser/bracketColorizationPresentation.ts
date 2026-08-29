import { type LanguageBracketPairs } from '../../../common/languages/languageBracketPairs.js';
import { type BracketColorizationSource as EditorBracketColorizationSource, type BracketColorizationSpan, type BracketGuide } from '../../../browser/viewparts/viewLines/viewLine.js';

/** Adapts common structural bracket levels into the editor's closed DOM vocabulary. */
export class BracketColorizationSource implements EditorBracketColorizationSource {
	constructor(private readonly bracketPairs: LanguageBracketPairs, private readonly colorizeBrackets = true) {}

	get textModel() {
		return this.bracketPairs.textModel;
	}

	getLineBrackets(lineIndex: number): readonly BracketColorizationSpan[] {
		if (!this.colorizeBrackets) return Object.freeze([]);
		return Object.freeze(this.bracketPairs.getLineBrackets(lineIndex).flatMap(bracket => !bracket.isInvalid ? [Object.freeze({
			startColumn: bracket.range.startColumn - 1,
			endColumn: bracket.range.endColumn - 1,
			level: bracket.nestingLevel % 6 + 1,
		})] : []));
	}

	getBracketGuides(startLineIndex: number, endLineIndexInclusive: number): readonly BracketGuide[] {
		return Object.freeze(this.bracketPairs.getBracketPairsInLineRange(startLineIndex, endLineIndexInclusive).map(bracket => Object.freeze({
			opening: bracket.pair!.opening,
			closing: bracket.pair!.closing,
			level: bracket.nestingLevel % 6 + 1,
		})));
	}
}
