import { Disposable } from "../../../../base/common/lifecycle.js";
import { type EditorViewport } from "../../../browser/view.js";
import { type LanguageTokenLineIndexPart } from "../common/languageTokenLineIndexPart.js";

/** Exposes tokenization readiness to the browser view without owning token production. */
export class TokenizationController extends Disposable {
	constructor(private readonly viewport: EditorViewport, private readonly tokenization: LanguageTokenLineIndexPart) {
		super();
		if (viewport.textModel !== tokenization.textModel) throw new TypeError("Stanza tokenization dependencies must share a text model");
		this._register(tokenization.onDidChange(() => this.update()));
		this.update();
	}

	private update(): void { this.viewport.element.classList.toggle("tokens-ready", this.tokenization.modelVersion === this.viewport.textModel.version && this.tokenization.tokenCount > 0); }
}
