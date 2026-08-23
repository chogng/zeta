import { DisposableOwner, type IDisposable } from "../../../../base/common/lifecycle.js";
import { type LanguageToken } from "../../../common/tokens/languageTokens.js";
import { type LanguageTokenLine, type LanguageTokenLineIndex } from "../../../common/tokens/languageTokenLineIndex.js";
import { type TextModel } from "../../../common/model/textModel.js";

/** Canonical tokenization model part consumed by semantic-token and view owners. */
export class TokenizationTextModelPart extends DisposableOwner {
  readonly onDidChange: (listener: (...args: any[]) => void) => IDisposable;

  constructor(private readonly index: LanguageTokenLineIndex) {
    super();
    this.onDidChange = listener => index.onDidChange(() => listener());
    this.own(index);
  }

  get textModel(): TextModel { return this.index.textModel; }
  get modelVersion(): number { return this.index.modelVersion; }
  get tokenCount(): number { return this.index.tokenCount; }
  get lines(): readonly LanguageTokenLine[] { return this.index.lines; }
  getLineTokens(lineIndex: number): readonly LanguageToken[] { return this.index.getLineTokens(lineIndex); }
}
