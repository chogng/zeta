import { addDisposableListener } from "../../../base/browser/dom.js";
import { DisposableOwner, ResettableDisposableGroup } from "../../../base/common/lifecycle.js";
import type { IWorkbenchSessionService } from "../../../workbench/services/sessions/common/sessionService.js";

/** Session picker shared by the Code and Academic dedicated workbenches. */
export class SessionsList extends DisposableOwner {
  readonly element: HTMLElement;
  private readonly heading: HTMLHeadingElement;
  private readonly newSessionButton: HTMLButtonElement;
  private readonly list: HTMLDivElement;
  private readonly itemListeners = this.own(new ResettableDisposableGroup());
  private readonly sessionService: IWorkbenchSessionService;

  constructor(ownerDocument: Document, sessionService: IWorkbenchSessionService, title: string, newSessionLabel: string) {
    super();
    this.sessionService = sessionService;
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
    this.own(addDisposableListener(this.newSessionButton, "click", () => sessionService.createUntitledSession(newSessionLabel)));
    this.own(sessionService.onDidChange(() => this.render()));
    this.render();
  }

  private render(): void {
    this.itemListeners.clear();
    const ownerDocument = this.element.ownerDocument;
    const items: HTMLElement[] = [];
    for (const session of this.sessionService.untitledSessions) {
      const button = sessionButton(ownerDocument, session.title || "New Session", this.sessionService.activeUntitledSession?.untitledSessionId === session.untitledSessionId);
      this.itemListeners.add(addDisposableListener(button, "click", () => this.sessionService.selectUntitledSession(session.untitledSessionId)));
      items.push(button);
    }
    for (const session of this.sessionService.sessions) {
      const thread = session.threads.find((candidate) => candidate.status === "active");
      if (!thread || session.status !== "active") continue;
      const button = sessionButton(ownerDocument, session.title || "Untitled Session", this.sessionService.active?.session.sessionId === session.sessionId && this.sessionService.active.threadId === thread.threadId);
      this.itemListeners.add(addDisposableListener(button, "click", () => this.sessionService.selectThread(session.sessionId, thread.threadId)));
      items.push(button);
    }
    if (items.length === 0) {
      const empty = ownerDocument.createElement("p");
      empty.className = "zeta-sessions-empty";
      empty.textContent = this.sessionService.state === "loading" ? "Loading sessions…" : "Create a session to begin.";
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
