import { type LanguageBracketColorizationIndex } from "../common/bracketColorization.js";
import { type BracketColorizationSource as EditorBracketColorizationSource, type BracketColorizationSpan } from "../../../browser/viewparts/semanticTokens/semanticTokenPresentation.js";

/** Adapts common lexical bracket nesting colors into Stanza's closed DOM vocabulary. */
export class BracketColorizationSource implements EditorBracketColorizationSource {
	constructor(private readonly index: LanguageBracketColorizationIndex) {}

	get textModel() {
		return this.index.textModel;
	}

	getLineBrackets(lineIndex: number): readonly BracketColorizationSpan[] {
		return Object.freeze(this.index.getLineColorizations(lineIndex).map(colorization => Object.freeze({
			startColumn: colorization.startColumn,
			endColumn: colorization.endColumn,
			level: colorization.level,
		})));
	}
}
