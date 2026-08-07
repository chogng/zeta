import { DisposableOwner } from "../../../../../base/common/lifecycle.js";
import { type EditorViewport } from "../../../browser/view/editorViewport.js";
import { type TokenizationTextModelPart } from "../common/tokenizationTextModelPart.js";

/** Exposes tokenization readiness to the browser view without owning token production. */
export class TokenizationController extends DisposableOwner {
  constructor(private readonly viewport: EditorViewport, private readonly tokenization: TokenizationTextModelPart) {
    super();
    if (viewport.textModel !== tokenization.textModel) throw new TypeError("Alpha tokenization dependencies must share a text model");
    this.own(tokenization.onDidChange(() => this.update()));
    this.update();
  }

  private update(): void { this.viewport.element.classList.toggle("tokens-ready", this.tokenization.modelVersion === this.viewport.textModel.version && this.tokenization.tokenCount > 0); }
}
