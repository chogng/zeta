import { Disposable, type IDisposable } from "../../../../base/common/lifecycle.js";
import { type LanguageToken } from "../../../common/tokens/languageTokens.js";
import { type LanguageTokenLine, type LanguageTokenLineIndex } from "../../../common/tokens/languageTokenLineIndex.js";
import { type TextModel } from "../../../common/model/textModel.js";

/** Zeta-specific projection that exposes one LanguageTokenLineIndex to editor contributions. */
export class LanguageTokenLineIndexPart extends Disposable {
	readonly onDidChange: (listener: (...args: any[]) => void) => IDisposable;

	constructor(private readonly index: LanguageTokenLineIndex) {
		super();
		this.onDidChange = listener => index.onDidChange(() => listener());
		this._register(index);
	}

	get textModel(): TextModel { return this.index.textModel; }
	get modelVersion(): number { return this.index.modelVersion; }
	get tokenCount(): number { return this.index.tokenCount; }
	get lines(): readonly LanguageTokenLine[] { return this.index.lines; }
	getLineTokens(lineIndex: number): readonly LanguageToken[] { return this.index.getLineTokens(lineIndex); }
}
