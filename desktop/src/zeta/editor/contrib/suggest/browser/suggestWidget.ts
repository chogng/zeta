import "./media/completionWidget.css";
import { addDisposableListener, reset, stopEvent } from "../../../../base/browser/dom.js";
import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import { type EditorSelectionController } from "../../../common/cursor/editorSelectionController.js";
import { LanguageCompletionDetailsStatus, type LanguageCompletionSessionState, LanguageCompletionSessionController } from "../common/suggestModel.js";
import { LanguageCompletionItemKind } from "../../../common/languages/completion/languageCompletions.js";
import { type EditorViewport } from "../../../browser/view/editorViewport.js";

let nextCompletionWidgetId = 1;

/** Projects one common completion session into Alpha-owned browser UI. */
export class CompletionWidget extends DisposableOwner {
  readonly element: HTMLDivElement;
  private readonly widgetId: string;
  private readonly previousAriaAutocomplete: string | null;
  private readonly previousAriaControls: string | null;
  private readonly previousAriaHasPopup: string | null;
  private readonly previousAriaActiveDescendant: string | null;

  constructor(
    private readonly inputElement: HTMLTextAreaElement,
    private readonly viewport: EditorViewport,
    private readonly selectionController: EditorSelectionController,
    private readonly session: LanguageCompletionSessionController,
  ) {
    super();
    try {
      if (
        viewport.textModel !== selectionController.textModel ||
        viewport.textModel !== session.textModel
      ) {
        throw new TypeError("Alpha completion widget dependencies must share one text model");
      }
    } catch (error) {
      this.dispose();
      throw error;
    }
    this.widgetId = `zeta-alpha-completion-${nextCompletionWidgetId++}`;
    this.previousAriaAutocomplete = inputElement.getAttribute("aria-autocomplete");
    this.previousAriaControls = inputElement.getAttribute("aria-controls");
    this.previousAriaHasPopup = inputElement.getAttribute("aria-haspopup");
    this.previousAriaActiveDescendant = inputElement.getAttribute("aria-activedescendant");
    const ownerDocument = viewport.element.ownerDocument;
    this.element = ownerDocument.createElement("div");
    this.element.id = this.widgetId;
    this.element.className = "zeta-alpha-editor-completion";
    this.element.setAttribute("role", "listbox");
    this.element.hidden = true;
    inputElement.setAttribute("aria-autocomplete", "none");
    inputElement.setAttribute("aria-controls", this.widgetId);
    inputElement.setAttribute("aria-haspopup", "listbox");
    viewport.element.append(this.element);
    this.defer(() => {
      this.element.remove();
      restoreAttribute(inputElement, "aria-autocomplete", this.previousAriaAutocomplete);
      restoreAttribute(inputElement, "aria-controls", this.previousAriaControls);
      restoreAttribute(inputElement, "aria-haspopup", this.previousAriaHasPopup);
      restoreAttribute(inputElement, "aria-activedescendant", this.previousAriaActiveDescendant);
    });
    this.own(session.onDidChange(() => this.render()));
    this.own(viewport.onDidChangeLayout(() => this.position()));
    this.own(addDisposableListener(inputElement, "keydown", event => this.handleKeydown(event)));
    this.own(addDisposableListener(inputElement, "blur", () => session.cancel()));
    this.own(addDisposableListener(inputElement, "compositionstart", () => session.cancel()));
    this.own(addDisposableListener<MouseEvent>(this.element, "mousedown", event => {
      const index = this.readOptionIndex(event);
      if (index === undefined || event.button !== 0) return;
      stopEvent(event);
      session.selectIndex(index);
      this.accept();
    }));
    this.render();
  }

  get visible(): boolean {
    return this.element.classList.contains("visible");
  }

  private handleKeydown(event: KeyboardEvent): void {
    if (
      event.defaultPrevented ||
      event.isComposing ||
      !this.readState() ||
      event.ctrlKey ||
      event.altKey ||
      event.metaKey
    ) {
      return;
    }
    switch (event.key) {
      case "ArrowDown":
        stopEvent(event);
        this.session.selectNext();
        return;
      case "ArrowUp":
        stopEvent(event);
        this.session.selectPrevious();
        return;
      case "Enter":
        if (event.shiftKey) return;
        stopEvent(event);
        this.accept();
        return;
      case "Tab":
        if (event.shiftKey) return;
        stopEvent(event);
        this.accept();
        return;
      case "Escape":
        stopEvent(event);
        this.session.cancel();
        return;
    }
  }

  private accept(): void {
    if (!this.session.acceptSelected()) return;
    this.viewport.revealPosition(this.selectionController.selections.primary.active);
    this.inputElement.focus({ preventScroll: true });
  }

  private render(): void {
    const state = this.readState();
    if (!state) {
      reset(this.element);
      this.element.classList.remove("visible");
      this.element.hidden = true;
      this.inputElement.setAttribute("aria-autocomplete", "none");
      this.inputElement.removeAttribute("aria-activedescendant");
      return;
    }
    const ownerDocument = this.element.ownerDocument;
    const fragment = ownerDocument.createDocumentFragment();
    for (let index = 0; index < state.items.length; index += 1) {
      const item = state.items[index]!;
      const option = ownerDocument.createElement("div");
      const kind = ownerDocument.createElement("span");
      const label = ownerDocument.createElement("span");
      const detail = ownerDocument.createElement("span");
      const documentation = ownerDocument.createElement("span");
      const focused = index === state.selectedIndex;
      const resolving = focused && state.detailsStatus === LanguageCompletionDetailsStatus.Loading;
      option.id = `${this.widgetId}-option-${index}`;
      option.className = "zeta-alpha-editor-completion-option";
      option.classList.toggle("focused", focused);
      option.classList.toggle("resolving", resolving);
      option.dataset.completionIndex = String(index);
      option.setAttribute("role", "option");
      option.setAttribute("aria-selected", String(focused));
      if (resolving) option.setAttribute("aria-busy", "true");
      kind.className = "zeta-alpha-editor-completion-kind";
      kind.setAttribute("aria-hidden", "true");
      kind.textContent = completionKindLabel(item.kind);
      label.className = "zeta-alpha-editor-completion-label";
      label.textContent = item.label;
      detail.className = "zeta-alpha-editor-completion-detail";
      detail.textContent = focused ? state.details.detail ?? "" : item.detail ?? "";
      option.append(kind, label, detail);
      if (focused && state.details.documentation !== undefined) {
        documentation.className = "zeta-alpha-editor-completion-documentation";
        documentation.textContent = state.details.documentation;
        option.append(documentation);
      }
      fragment.append(option);
    }
    reset(this.element, fragment);
    this.element.hidden = false;
    this.element.classList.add("visible");
    this.inputElement.setAttribute("aria-autocomplete", "list");
    this.inputElement.setAttribute("aria-activedescendant", `${this.widgetId}-option-${state.selectedIndex}`);
    this.position(state);
  }

  private position(state = this.readState()): void {
    if (!state) return;
    const coordinates = this.viewport.getPositionContentCoordinates(state.position);
    this.element.style.left = `${coordinates.left}px`;
    this.element.style.top = `${coordinates.top + coordinates.height}px`;
  }

  private readState(): LanguageCompletionSessionState | undefined {
    try {
      return this.session.state;
    } catch (error) {
      if (error instanceof ReferenceError) return undefined;
      throw error;
    }
  }

  private readOptionIndex(event: MouseEvent): number | undefined {
    const target = event.target;
    const ElementConstructor = this.element.ownerDocument.defaultView?.Element;
    if (!ElementConstructor || !(target instanceof ElementConstructor)) return undefined;
    const option = (target as Element).closest<HTMLElement>(".zeta-alpha-editor-completion-option");
    if (!option || !this.element.contains(option)) return undefined;
    const index = Number(option.dataset.completionIndex);
    return Number.isSafeInteger(index) ? index : undefined;
  }
}

function completionKindLabel(kind: LanguageCompletionItemKind): string {
  switch (kind) {
    case LanguageCompletionItemKind.Text: return "Text";
    case LanguageCompletionItemKind.Method: return "Method";
    case LanguageCompletionItemKind.Function: return "Function";
    case LanguageCompletionItemKind.Constructor: return "Constructor";
    case LanguageCompletionItemKind.Field: return "Field";
    case LanguageCompletionItemKind.Variable: return "Variable";
    case LanguageCompletionItemKind.Class: return "Class";
    case LanguageCompletionItemKind.Interface: return "Interface";
    case LanguageCompletionItemKind.Module: return "Module";
    case LanguageCompletionItemKind.Property: return "Property";
    case LanguageCompletionItemKind.Unit: return "Unit";
    case LanguageCompletionItemKind.Value: return "Value";
    case LanguageCompletionItemKind.Enum: return "Enum";
    case LanguageCompletionItemKind.Keyword: return "Keyword";
    case LanguageCompletionItemKind.Snippet: return "Snippet";
    case LanguageCompletionItemKind.File: return "File";
    case LanguageCompletionItemKind.Folder: return "Folder";
    case LanguageCompletionItemKind.Reference: return "Reference";
    case LanguageCompletionItemKind.TypeParameter: return "Type";
  }
}

function restoreAttribute(element: Element, name: string, value: string | null): void {
  if (value === null) element.removeAttribute(name);
  else element.setAttribute(name, value);
}
