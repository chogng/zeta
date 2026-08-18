import "./sessionsControls.css";
import "./sessionsList.css";
import { addDisposableListener } from "../../../base/browser/dom.js";
import { DisposableOwner, ResettableDisposableGroup } from "../../../base/common/lifecycle.js";
import type { ISessionsViewService } from "../../services/view/common/sessionsViewService.js";
import type { ISessionsManagementService } from "../../services/sessions/common/sessionsManagementService.js";

/** Session picker owned by the dedicated Sessions Workbench sidebar. */
export class SessionsList extends DisposableOwner {
  readonly element: HTMLElement;
  private readonly heading: HTMLHeadingElement;
  private readonly newSessionButton: HTMLButtonElement;
  private readonly list: HTMLDivElement;
  private readonly itemListeners = this.own(new ResettableDisposableGroup());
  private readonly sessionService: ISessionsManagementService;
  private readonly viewService: ISessionsViewService;

  constructor(ownerDocument: Document, sessionService: ISessionsManagementService, viewService: ISessionsViewService, title: string, newSessionLabel: string) {
    super();
    this.sessionService = sessionService;
    this.viewService = viewService;
    this.element = ownerDocument.createElement("section");
    this.element.className = "zeta-sessions-list";
    this.heading = ownerDocument.createElement("h2");
    this.heading.textContent = title;
    this.newSessionButton = ownerDocument.createElement("button");
    this.newSessionButton.type = "button";
    this.newSessionButton.className = "zeta-sessions-button zeta-sessions-primary-button";
    this.newSessionButton.textContent = newSessionLabel;
    this.list = ownerDocument.createElement("div");
    this.list.className = "zeta-sessions-list-items";
    this.element.append(this.heading, this.newSessionButton, this.list);
    this.own(addDisposableListener(this.newSessionButton, "click", () => viewService.openNewSession(newSessionLabel)));
    this.own(viewService.onDidChange(() => this.render()));
    this.render();
  }

  focus(): void {
    const firstItem = this.list.querySelector<HTMLButtonElement>("button");
    if (firstItem) firstItem.focus();
    else this.newSessionButton.focus();
  }

  private render(): void {
    this.itemListeners.clear();
    const ownerDocument = this.element.ownerDocument;
    const items: HTMLElement[] = [];
    const activeSelection = this.viewService.activeSelection;
    for (const session of this.sessionService.untitledSessions) {
      const selected = activeSelection?.kind === "untitled" && activeSelection.session.untitledSessionId === session.untitledSessionId;
      const button = sessionButton(ownerDocument, session.title || "New Session", selected);
      this.itemListeners.add(addDisposableListener(button, "click", () => this.viewService.openUntitledSession(session.untitledSessionId)));
      items.push(button);
    }
    for (const session of this.sessionService.sessions) {
      const current = activeSelection?.kind === "session" && activeSelection.active.session.sessionId === session.sessionId ? activeSelection.active : undefined;
      const thread = current
        ? session.threads.find(candidate => candidate.threadId === current.threadId && candidate.status === "active")
        : session.threads.find(candidate => candidate.status === "active" && candidate.origin.type === "root") ?? session.threads.find(candidate => candidate.status === "active");
      if (!thread || session.status !== "active") continue;
      const button = sessionButton(ownerDocument, session.title || "Untitled Session", current !== undefined);
      this.itemListeners.add(addDisposableListener(button, "click", () => this.viewService.openSession(session.sessionId, thread.threadId)));
      items.push(button);
    }
    if (items.length === 0) {
      const empty = ownerDocument.createElement("p");
      empty.className = "zeta-sessions-empty";
      empty.textContent = this.sessionService.state === "loading"
        ? "Loading sessions…"
        : this.sessionService.error ?? "Create a session to begin.";
      items.push(empty);
    }
    this.list.replaceChildren(...items);
  }
}

function sessionButton(ownerDocument: Document, title: string, selected: boolean): HTMLButtonElement {
  const button = ownerDocument.createElement("button");
  button.type = "button";
  button.className = "zeta-sessions-list-item";
  button.classList.toggle("selected", selected);
  button.setAttribute("aria-current", selected ? "page" : "false");
  button.textContent = title;
  return button;
}
