import { type IDisposable } from "../../../../base/common/lifecycle.js";
import { type LanguageToken } from "../../../common/tokens/languageTokens.js";
import { type LanguageTokenLine } from "../../../common/tokens/languageTokenLineIndex.js";
import { type TextModel } from "../../../common/model/textModel.js";

/** Stable common semantic-token source shape; browser presentation stays outside this contract. */
export interface SemanticTokensModelPart {
  readonly textModel: TextModel;
  readonly onDidChange: (listener: (...args: any[]) => void) => IDisposable;
  readonly lines: readonly LanguageTokenLine[];
  getLineTokens(lineIndex: number): readonly LanguageToken[];
}
