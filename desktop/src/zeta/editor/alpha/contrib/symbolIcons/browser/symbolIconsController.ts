import "./media/symbolIcons.css";
import { DisposableOwner } from "../../../../../base/common/lifecycle.js";
import { type AlphaEditorViewport } from "../../../browser/view/editorViewport.js";
import { type DocumentSymbolService, type LanguageDocumentSymbol } from "../../documentSymbols/common/documentSymbols.js";

/** Projects document-symbol kinds into small, feature-owned gutter icons. */
export class AlphaSymbolIconsController extends DisposableOwner {
  private symbols: readonly LanguageDocumentSymbol[] = [];
  private request: AbortController | undefined;

  constructor(private readonly viewport: AlphaEditorViewport, private readonly service: DocumentSymbolService, private readonly languageId: string, private readonly onError: (error: unknown) => void = error => console.error("Alpha symbol icons failed", error)) {
    super();
    if (service.textModel !== viewport.textModel) throw new TypeError("Alpha symbol icon dependencies must share a text model");
    this.own(viewport.onDidChangeLayout(() => this.render()));
    this.own(viewport.textModel.onDidChange(() => void this.refresh()));
    this.defer(() => this.request?.abort());
    void this.refresh();
  }

  private async refresh(): Promise<void> {
    this.request?.abort();
    const request = this.request = new AbortController();
    try {
      this.symbols = await this.service.provideDocumentSymbols(this.languageId, request.signal);
      if (!request.signal.aborted) this.render();
    } catch (error) {
      if (!request.signal.aborted) this.onError(error);
    }
  }

  private render(): void {
    for (const element of [...this.viewport.element.querySelectorAll<HTMLElement>(".zeta-alpha-editor-symbol-icon")]) element.remove();
    for (const symbol of flatten(this.symbols)) {
      const line = this.viewport.element.querySelector<HTMLElement>(`.zeta-alpha-editor-line[data-logical-line-index="${symbol.selectionRange.start.lineIndex}"]`);
      if (!line) continue;
      const icon = this.viewport.element.ownerDocument.createElement("span");
      icon.className = "zeta-alpha-editor-symbol-icon";
      icon.textContent = symbolIcon(symbol.kind);
      icon.title = symbol.detail ? `${symbol.name}: ${symbol.detail}` : symbol.name;
      icon.setAttribute("aria-label", icon.title);
      line.prepend(icon);
    }
  }
}

function flatten(symbols: readonly LanguageDocumentSymbol[]): readonly LanguageDocumentSymbol[] { return symbols.flatMap(symbol => [symbol, ...flatten(symbol.children ?? [])]); }
function symbolIcon(kind: string | number): string { const value = String(kind).toLowerCase(); return value.includes("class") || value === "5" ? "◆" : value.includes("function") || value === "12" ? "ƒ" : value.includes("property") || value === "10" ? "·" : "◇"; }
