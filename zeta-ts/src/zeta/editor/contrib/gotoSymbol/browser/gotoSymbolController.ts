import "./media/gotoSymbol.css";
import { addDisposableListener, stopEvent, h } from "../../../../base/browser/dom.js";
import { Disposable, DisposableStore, toDisposable } from "../../../../base/common/lifecycle.js";
import { Selection } from "../../../common/core/selection.js";
import { SelectionSet } from "../../../common/cursor/selectionSet.js";
import { type CursorsController } from "../../../common/cursor/cursor.js";
import { type GotoSymbolService, type LanguageSymbolMatch } from "../common/languageDocumentSymbolSearch.js";
import { type EditorViewport } from "../../../browser/view.js";

/** Owns editor-local document-symbol quick navigation (Ctrl/Cmd+Shift+O). */
export class GotoSymbolController extends Disposable {
	private readonly element: HTMLDivElement;
	private readonly queryInput: HTMLInputElement;
	private readonly list: HTMLDivElement;
	private readonly itemListeners = this._register(new DisposableStore());
	private request: AbortController | undefined;
	private matches: readonly LanguageSymbolMatch[] = [];

	constructor(private readonly input: HTMLElement, private readonly viewport: EditorViewport, private readonly selections: CursorsController, private readonly service: GotoSymbolService, private readonly languageId: string, private readonly onError: (error: unknown) => void = error => console.error("Stanza goto symbol failed", error)) {
		super();
		const ownerDocument = viewport.element.ownerDocument;
		this.element = h(ownerDocument, "div");
		this.element.className = "stanza-editor-goto-symbol";
		this.element.hidden = true;
		this.element.setAttribute("role", "dialog");
		this.element.setAttribute("aria-label", "Go to Symbol");
		this.queryInput = h(ownerDocument, "input");
		this.queryInput.className = "stanza-editor-goto-symbol-input";
		this.queryInput.type = "search";
		this.queryInput.placeholder = "Type a symbol name";
		this.queryInput.setAttribute("aria-label", "Symbol query");
		this.list = h(ownerDocument, "div");
		this.list.className = "stanza-editor-goto-symbol-list";
		this.list.setAttribute("role", "listbox");
		this.element.append(this.queryInput, this.list);
		viewport.element.append(this.element);
		this._register(toDisposable(() => {
			this.request?.abort();
			this.element.remove();
		}));
		this._register(addDisposableListener(input, "keydown", event => {
			if (event.defaultPrevented || event.isComposing || event.altKey || !event.shiftKey || (!event.ctrlKey && !event.metaKey) || event.key.toLowerCase() !== "o") return;
			stopEvent(event);
			this.open();
		}));
		this._register(addDisposableListener(this.element, "keydown", event => {
			if (event.key !== "Escape") return;
			stopEvent(event);
			this.close();
		}));
		this._register(addDisposableListener(this.queryInput, "input", () => void this.refresh()));
		this._register(viewport.onDidChangeLayout(() => this.position()));
	}

	private open(): void {
		this.element.hidden = false;
		this.queryInput.value = "";
		this.position();
		this.queryInput.focus({ preventScroll: true });
		void this.refresh();
	}

	private async refresh(): Promise<void> {
		this.request?.abort();
		const request = this.request = new AbortController();
		try {
			this.matches = await this.service.query(this.languageId, this.queryInput.value, request.signal);
			if (!request.signal.aborted) this.render();
		} catch (error) {
			if (!request.signal.aborted) this.onError(error);
		}
	}

	private render(): void {
		this.itemListeners.clear();
		this.list.replaceChildren(...this.matches.map((match, index) => {
			const item = h(this.list.ownerDocument, "button");
			item.className = "stanza-editor-goto-symbol-item";
			item.type = "button";
			item.setAttribute("role", "option");
			item.textContent = match.symbol.detail ? `${match.symbol.name} — ${match.symbol.detail}` : match.symbol.name;
			item.tabIndex = index === 0 ? 0 : -1;
			this.itemListeners.add(addDisposableListener(item, "click", () => this.select(match)));
			return item;
		}));
	}

	private select(match: LanguageSymbolMatch): void {
		this.selections.setSelections(SelectionSet.single(Selection.fromPositions(match.symbol.selectionRange.getStartPosition(), match.symbol.selectionRange.getEndPosition())));
		this.viewport.revealPosition(match.position);
		this.close();
	}

	private position(): void {
		if (this.element.hidden) return;
		const layout = this.viewport.viewportLayout;
		this.element.style.left = `${layout.scrollPosition.left + 8}px`;
		this.element.style.top = `${layout.scrollPosition.top + 8}px`;
	}

	private close(): void {
		this.request?.abort();
		this.request = undefined;
		this.element.hidden = true;
		this.itemListeners.clear();
		this.list.replaceChildren();
		this.input.focus({ preventScroll: true });
	}
}
