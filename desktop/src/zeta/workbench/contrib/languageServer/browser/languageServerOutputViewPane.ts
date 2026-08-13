import { addDisposableListener } from "../../../../base/browser/dom.js";
import { ViewPane, type IViewPaneOptions } from "../../../browser/parts/views/viewPane.js";
import { type ILanguageServerStatusService } from "../../../services/language/common/languageServerStatusService.js";
import "./media/languageServerOutput.css";

/** Language-server log and progress projection hosted by the Output panel. */
export class LanguageServerOutputViewPane extends ViewPane {
  private readonly content: HTMLDivElement;

  constructor(options: IViewPaneOptions, private readonly status: ILanguageServerStatusService) {
    super(options);
    this.contentElement.classList.add("zeta-language-server-output");
    const toolbar = options.ownerDocument.createElement("div");
    toolbar.className = "zeta-language-server-output-toolbar";
    const clear = options.ownerDocument.createElement("button");
    clear.type = "button";
    clear.textContent = "Clear";
    clear.setAttribute("aria-label", "Clear language-server output");
    toolbar.append(clear);
    this.content = options.ownerDocument.createElement("div");
    this.content.className = "zeta-language-server-output-content";
    this.content.setAttribute("role", "log");
    this.content.setAttribute("aria-live", "polite");
    this.contentElement.append(toolbar, this.content);
    this.own(addDisposableListener(clear, "click", () => status.clearLog()));
    this.own(status.onDidChange(() => this.render()));
    this.render();
  }

  private render(): void {
    const document = this.element.ownerDocument;
    const progress = this.status.getProgress().map(item => {
      const row = document.createElement("div");
      row.className = "zeta-language-server-output-progress";
      row.textContent = `[${item.server}] ${item.title}${item.percentage === undefined ? "" : ` ${item.percentage}%`}${item.message ? ` - ${item.message}` : ""}`;
      return row;
    });
    const logs = this.status.getLogEntries().map(entry => {
      const row = document.createElement("div");
      row.className = `zeta-language-server-output-row ${entry.severity}`;
      row.textContent = `[${entry.server}] ${entry.message}`;
      return row;
    });
    if (progress.length === 0 && logs.length === 0) {
      const empty = document.createElement("div");
      empty.className = "zeta-language-server-output-empty";
      empty.textContent = "No language-server output is available.";
      this.content.replaceChildren(empty);
      return;
    }
    this.content.replaceChildren(...progress, ...logs);
    this.content.scrollTop = this.content.scrollHeight;
  }
}
