import { addDisposableListener } from "../../../base/browser/dom.js";
import { Emitter, type Event } from "../../../base/common/event.js";
import { DisposableOwner } from "../../../base/common/lifecycle.js";
import type { IBrowserViewApi } from "../../../platform/browser/common/browserView.js";
import { SessionBrowserSurface } from "../common/sessionBrowserSurface.js";
import type { ResearchLibraryItem } from "./academicLibraryPane.js";

type AcademicWorkspaceSurface = "reader" | "browser" | "draft";

/** Central research canvas: read a source, browse the web, or draft with the agent. */
export class AcademicResearchWorkspace extends DisposableOwner {
  readonly element: HTMLElement;
  readonly onDidRequestWritingHelp: Event<string>;
  private readonly _onDidRequestWritingHelp = this.own(new Emitter<string>());
  private readonly tabs = new Map<AcademicWorkspaceSurface, HTMLButtonElement>();
  private readonly reader: HTMLDivElement;
  private readonly readerTitle: HTMLHeadingElement;
  private readonly readerDetail: HTMLParagraphElement;
  private readonly pdf: HTMLEmbedElement;
  private readonly browser: SessionBrowserSurface;
  private readonly draft: HTMLDivElement;
  private readonly draftInput: HTMLTextAreaElement;
  private pdfUrl: string | undefined;

  constructor(ownerDocument: Document, browserViewApi: IBrowserViewApi | undefined) {
    super();
    this.onDidRequestWritingHelp = this._onDidRequestWritingHelp.event;
    this.element = ownerDocument.createElement("section");
    this.element.className = "zeta-academic-workspace";
    const header = ownerDocument.createElement("div");
    header.className = "zeta-academic-workspace-tabs";
    for (const [surface, label] of [["reader", "Read"], ["browser", "Browse"], ["draft", "Draft"]] as const) {
      const button = ownerDocument.createElement("button");
      button.type = "button";
      button.className = "zeta-academic-workspace-tab";
      button.textContent = label;
      button.setAttribute("role", "tab");
      this.tabs.set(surface, button);
      this.own(addDisposableListener(button, "click", () => this.show(surface)));
      header.append(button);
    }
    this.reader = ownerDocument.createElement("div");
    this.reader.className = "zeta-academic-reader";
    this.readerTitle = ownerDocument.createElement("h1");
    this.readerTitle.textContent = "Select a source from your library";
    this.readerDetail = ownerDocument.createElement("p");
    this.readerDetail.textContent = "Imported PDFs stay beside the writing and agent workflow.";
    this.pdf = ownerDocument.createElement("embed");
    this.pdf.type = "application/pdf";
    this.pdf.className = "zeta-academic-pdf";
    this.pdf.hidden = true;
    this.reader.append(this.readerTitle, this.readerDetail, this.pdf);
    this.browser = this.own(new SessionBrowserSurface(ownerDocument, browserViewApi));
    this.draft = ownerDocument.createElement("div");
    this.draft.className = "zeta-academic-draft";
    const draftHeading = ownerDocument.createElement("h1");
    draftHeading.textContent = "Writing draft";
    this.draftInput = ownerDocument.createElement("textarea");
    this.draftInput.className = "zeta-academic-draft-input";
    this.draftInput.placeholder = "Write an outline, argument, or paragraph. Ask the agent to improve it when you are ready.";
    const askAgent = ownerDocument.createElement("button");
    askAgent.type = "button";
    askAgent.className = "zeta-sessions-button zeta-sessions-primary-button";
    askAgent.textContent = "Ask writing agent";
    this.draft.append(draftHeading, this.draftInput, askAgent);
    this.own(addDisposableListener(askAgent, "click", () => {
      const draft = this.draftInput.value.trim();
      this._onDidRequestWritingHelp.fire(draft
        ? `Help improve this academic draft. Preserve my intent and identify claims that need citations:\n\n${draft}`
        : "Help me create an evidence-led academic writing outline. Ask for my research question before drafting.");
    }));
    this.element.append(header, this.reader, this.browser.element, this.draft);
    this.defer(() => this.releasePdfUrl());
    this.show("reader");
  }

  showSource(item: ResearchLibraryItem | undefined): void {
    if (!item) return;
    this.releasePdfUrl();
    this.readerTitle.textContent = item.title;
    this.readerDetail.textContent = item.kind === "pdf" ? "Reading imported PDF" : `Imported ${item.kind === "bibtex" ? "BibTeX" : "RIS"} reference`;
    if (item.kind === "pdf") {
      this.pdfUrl = URL.createObjectURL(item.file);
      this.pdf.src = this.pdfUrl;
      this.pdf.hidden = false;
    } else {
      this.pdf.removeAttribute("src");
      this.pdf.hidden = true;
    }
    this.show("reader");
  }

  private show(surface: AcademicWorkspaceSurface): void {
    for (const [candidate, button] of this.tabs) {
      const selected = candidate === surface;
      button.classList.toggle("checked", selected);
      button.setAttribute("aria-selected", String(selected));
    }
    this.reader.hidden = surface !== "reader";
    this.browser.element.hidden = surface !== "browser";
    this.draft.hidden = surface !== "draft";
    this.browser.setVisible(surface === "browser");
    if (surface === "draft") this.draftInput.focus({ preventScroll: true });
  }

  private releasePdfUrl(): void {
    if (!this.pdfUrl) return;
    URL.revokeObjectURL(this.pdfUrl);
    this.pdfUrl = undefined;
  }
}
