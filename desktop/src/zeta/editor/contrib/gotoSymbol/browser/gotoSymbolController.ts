import "./media/gotoSymbol.css";
import { addDisposableListener, stopEvent } from "../../../../base/browser/dom.js";
import { DisposableOwner, ResettableDisposableGroup } from "../../../../base/common/lifecycle.js";
import { TextSelection, TextSelectionSet } from "../../../common/core/selection.js";
import { type EditorSelectionController } from "../../../common/cursor/editorSelectionController.js";
import { type GotoSymbolService, type LanguageSymbolMatch } from "../common/gotoSymbol.js";
import { type EditorViewport } from "../../../browser/view/editorViewport.js";

/** Owns editor-local document-symbol quick navigation (Ctrl/Cmd+Shift+O). */
export class GotoSymbolController extends DisposableOwner {
  private readonly element: HTMLDivElement;
  private readonly queryInput: HTMLInputElement;
  private readonly list: HTMLDivElement;
  private readonly itemListeners = this.own(new ResettableDisposableGroup());
  private request: AbortController | undefined;
  private matches: readonly LanguageSymbolMatch[] = [];

  constructor(private readonly input: HTMLTextAreaElement, private readonly viewport: EditorViewport, private readonly selections: EditorSelectionController, private readonly service: GotoSymbolService, private readonly languageId: string, private readonly onError: (error: unknown) => void = error => console.error("Alpha goto symbol failed", error)) {
    super();
    const ownerDocument = viewport.element.ownerDocument;
    this.element = ownerDocument.createElement("div");
    this.element.className = "zeta-alpha-editor-goto-symbol";
    this.element.hidden = true;
    this.element.setAttribute("role", "dialog");
    this.element.setAttribute("aria-label", "Go to Symbol");
    this.queryInput = ownerDocument.createElement("input");
    this.queryInput.className = "zeta-alpha-editor-goto-symbol-input";
    this.queryInput.type = "search";
    this.queryInput.placeholder = "Type a symbol name";
    this.queryInput.setAttribute("aria-label", "Symbol query");
    this.list = ownerDocument.createElement("div");
    this.list.className = "zeta-alpha-editor-goto-symbol-list";
    this.list.setAttribute("role", "listbox");
    this.element.append(this.queryInput, this.list);
    viewport.element.append(this.element);
    this.defer(() => {
      this.request?.abort();
      this.element.remove();
    });
    this.own(addDisposableListener(input, "keydown", event => {
      if (event.defaultPrevented || event.isComposing || event.altKey || !event.shiftKey || (!event.ctrlKey && !event.metaKey) || event.key.toLowerCase() !== "o") return;
      stopEvent(event);
      this.open();
    }));
    this.own(addDisposableListener(this.element, "keydown", event => {
      if (event.key !== "Escape") return;
      stopEvent(event);
      this.close();
    }));
    this.own(addDisposableListener(this.queryInput, "input", () => void this.refresh()));
    this.own(viewport.onDidChangeLayout(() => this.position()));
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
      const item = this.list.ownerDocument.createElement("button");
      item.className = "zeta-alpha-editor-goto-symbol-item";
      item.type = "button";
      item.setAttribute("role", "option");
      item.textContent = match.symbol.detail ? `${match.symbol.name} — ${match.symbol.detail}` : match.symbol.name;
      item.tabIndex = index === 0 ? 0 : -1;
      this.itemListeners.add(addDisposableListener(item, "click", () => this.select(match)));
      return item;
    }));
  }

  private select(match: LanguageSymbolMatch): void {
    this.selections.setSelections(TextSelectionSet.single(TextSelection.from(match.symbol.selectionRange.start, match.symbol.selectionRange.end)));
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
