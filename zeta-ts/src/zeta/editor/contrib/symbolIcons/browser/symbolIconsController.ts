import "./media/symbolIcons.css";
import { Disposable, toDisposable } from "../../../../base/common/lifecycle.js";
import { type EditorViewport } from "../../../browser/view.js";
import { type DocumentSymbolService, type LanguageDocumentSymbol } from "../../documentSymbols/common/documentSymbols.js";
import { h } from "../../../../base/browser/dom.js";

/** Projects document-symbol kinds into small, feature-owned gutter icons. */
export class SymbolIconsController extends Disposable {
	private symbols: readonly LanguageDocumentSymbol[] = [];
	private request: AbortController | undefined;

	constructor(private readonly viewport: EditorViewport, private readonly service: DocumentSymbolService, private readonly languageId: string, private readonly onError: (error: unknown) => void = error => console.error("Stanza symbol icons failed", error)) {
		super();
		if (service.textModel !== viewport.textModel) throw new TypeError("Stanza symbol icon dependencies must share a text model");
		this._register(viewport.onDidChangeLayout(() => this.render()));
		this._register(viewport.textModel.onDidChange(() => void this.refresh()));
		this._register(toDisposable(() => this.request?.abort()));
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
		for (const element of [...this.viewport.element.querySelectorAll<HTMLElement>(".stanza-editor-symbol-icon")]) element.remove();
		for (const symbol of flatten(this.symbols)) {
			const line = this.viewport.element.querySelector<HTMLElement>(`.stanza-editor-line[data-logical-line-index="${symbol.selectionRange.start.lineIndex}"]`);
			if (!line) continue;
			const icon = h(this.viewport.element.ownerDocument, "span");
			icon.className = "stanza-editor-symbol-icon";
			icon.textContent = symbolIcon(symbol.kind);
			icon.title = symbol.detail ? `${symbol.name}: ${symbol.detail}` : symbol.name;
			icon.setAttribute("aria-label", icon.title);
			const lineMargin = line.querySelector(".stanza-editor-line-margin");
			if (!lineMargin) throw new Error("Rendered editor line is missing its margin");
			lineMargin.after(icon);
		}
	}
}

function flatten(symbols: readonly LanguageDocumentSymbol[]): readonly LanguageDocumentSymbol[] { return symbols.flatMap(symbol => [symbol, ...flatten(symbol.children ?? [])]); }
function symbolIcon(kind: string | number): string { const value = String(kind).toLowerCase(); return value.includes("class") || value === "5" ? "◆" : value.includes("function") || value === "12" ? "ƒ" : value.includes("property") || value === "10" ? "·" : "◇"; }
