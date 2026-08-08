import { addDisposableListener } from "../../../base/browser/dom.js";
import { Emitter, type Event } from "../../../base/common/event.js";
import { DisposableOwner, ResettableDisposableGroup } from "../../../base/common/lifecycle.js";

export type ResearchLibraryItemKind = "pdf" | "bibtex" | "ris";

/** A local source imported into the Academic Sessions research library. */
export interface ResearchLibraryItem {
  readonly id: string;
  readonly kind: ResearchLibraryItemKind;
  readonly title: string;
  readonly file: File;
}

/** Zotero-style local import and source selection pane for Academic Sessions. */
export class AcademicLibraryPane extends DisposableOwner {
  readonly element: HTMLElement;
  readonly onDidSelectItem: Event<ResearchLibraryItem | undefined>;
  private readonly _onDidSelectItem = this.own(new Emitter<ResearchLibraryItem | undefined>());
  private readonly fileInput: HTMLInputElement;
  private readonly list: HTMLDivElement;
  private readonly empty: HTMLParagraphElement;
  private readonly itemListeners = this.own(new ResettableDisposableGroup());
  private items: readonly ResearchLibraryItem[] = [];
  private selectedItemId: string | undefined;

  constructor(ownerDocument: Document) {
    super();
    this.onDidSelectItem = this._onDidSelectItem.event;
    this.element = ownerDocument.createElement("section");
    this.element.className = "zeta-academic-library";
    const header = ownerDocument.createElement("div");
    header.className = "zeta-academic-section-header";
    const title = ownerDocument.createElement("h2");
    title.textContent = "Library";
    const importButton = ownerDocument.createElement("button");
    importButton.type = "button";
    importButton.className = "zeta-sessions-button zeta-sessions-primary-button";
    importButton.textContent = "Import";
    header.append(title, importButton);
    this.fileInput = ownerDocument.createElement("input");
    this.fileInput.type = "file";
    this.fileInput.accept = ".pdf,.bib,.ris,application/pdf,text/plain";
    this.fileInput.multiple = true;
    this.fileInput.hidden = true;
    this.list = ownerDocument.createElement("div");
    this.list.className = "zeta-academic-library-items";
    this.empty = ownerDocument.createElement("p");
    this.empty.className = "zeta-sessions-empty";
    this.empty.textContent = "Import PDFs, BibTeX, or RIS references.";
    this.element.append(header, this.fileInput, this.list, this.empty);
    this.own(addDisposableListener(importButton, "click", () => this.fileInput.click()));
    this.own(addDisposableListener(this.fileInput, "change", () => {
      const files = [...(this.fileInput.files ?? [])];
      void this.importFiles(files);
      this.fileInput.value = "";
    }));
  }

  get selectedItem(): ResearchLibraryItem | undefined {
    return this.items.find((item) => item.id === this.selectedItemId);
  }

  async importFiles(files: readonly File[]): Promise<void> {
    const imported = files.flatMap((file) => toLibraryItem(file));
    if (imported.length === 0) return;
    this.items = [...imported, ...this.items];
    this.render();
    this.selectItem(imported[0]?.id);
  }

  private selectItem(itemId: string | undefined): void {
    if (itemId === this.selectedItemId) return;
    this.selectedItemId = itemId;
    this.render();
    this._onDidSelectItem.fire(this.selectedItem);
  }

  private render(): void {
    this.itemListeners.clear();
    const ownerDocument = this.element.ownerDocument;
    const items = this.items.map((item) => {
      const button = ownerDocument.createElement("button");
      button.type = "button";
      button.className = "zeta-academic-library-item";
      const selected = item.id === this.selectedItemId;
      button.classList.toggle("selected", selected);
      button.setAttribute("aria-current", selected ? "page" : "false");
      const kind = ownerDocument.createElement("span");
      kind.className = "zeta-academic-library-kind";
      kind.textContent = item.kind === "pdf" ? "PDF" : item.kind === "bibtex" ? "BIB" : "RIS";
      const title = ownerDocument.createElement("span");
      title.className = "zeta-academic-library-title";
      title.textContent = item.title;
      button.append(kind, title);
      this.itemListeners.add(addDisposableListener(button, "click", () => this.selectItem(item.id)));
      return button;
    });
    this.list.replaceChildren(...items);
    this.empty.hidden = items.length > 0;
  }
}

function toLibraryItem(file: File): ResearchLibraryItem[] {
  const extension = file.name.split(".").at(-1)?.toLowerCase();
  const kind = extension === "pdf" || file.type.toLowerCase() === "application/pdf"
    ? "pdf"
    : extension === "bib"
      ? "bibtex"
      : extension === "ris"
        ? "ris"
        : undefined;
  if (!kind) return [];
  const title = file.name.replace(/\.[^.]+$/, "").replace(/[_-]+/g, " ").trim() || "Untitled reference";
  return [{ id: `${kind}:${file.name}:${file.lastModified}:${file.size}`, kind, title, file }];
}
