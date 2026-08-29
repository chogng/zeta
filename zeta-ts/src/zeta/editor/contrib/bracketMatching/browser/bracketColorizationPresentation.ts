import { type LanguageBracketPairs } from '../../../common/languages/languageBracketPairs.js';
import { type BracketColorizationSource as EditorBracketColorizationSource, type BracketColorizationSpan } from '../../../browser/viewparts/viewLines/semanticTokenPresentation.js';

/** Adapts common structural bracket levels into the editor's closed DOM vocabulary. */
export class BracketColorizationSource implements EditorBracketColorizationSource {
	constructor(private readonly bracketPairs: LanguageBracketPairs) {}

	get textModel() {
		return this.bracketPairs.textModel;
	}

	getLineBrackets(lineIndex: number): readonly BracketColorizationSpan[] {
		return Object.freeze(this.bracketPairs.getLineBrackets(lineIndex).flatMap(bracket => !bracket.isInvalid ? [Object.freeze({
			startColumn: bracket.range.start.columnIndex,
			endColumn: bracket.range.end.columnIndex,
			level: bracket.nestingLevel % 6 + 1,
		})] : []));
	}
}
