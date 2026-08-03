import { type LanguageBracketColorizationIndex } from "../common/languageBracketColorization.js";
import { type AlphaBracketColorizationSpan } from "./semanticTokenPresentation.js";

/** Adapts common lexical bracket nesting colors into Alpha's closed DOM vocabulary. */
export class AlphaBracketColorizationSource {
  constructor(private readonly index: LanguageBracketColorizationIndex) {}

  get textModel() {
    return this.index.textModel;
  }

  getLineBrackets(lineIndex: number): readonly AlphaBracketColorizationSpan[] {
    return Object.freeze(this.index.getLineColorizations(lineIndex).map(colorization => Object.freeze({
      startColumn: colorization.startColumn,
      endColumn: colorization.endColumn,
      level: colorization.level,
    })));
  }
}
