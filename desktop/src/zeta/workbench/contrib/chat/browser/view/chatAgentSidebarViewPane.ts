import { ViewPane, type IViewPaneOptions } from "../../../../browser/parts/views/viewPane.js";
import type { IWorkbenchSessionService, Session, SessionThread } from "../../../../services/sessions/common/sessionService.js";

/** Lists durable Chat sessions available to the Workbench Agent Sidebar. */
export class ChatAgentSidebarViewPane extends ViewPane {
  private readonly sessionService: IWorkbenchSessionService;

  constructor(options: IViewPaneOptions, sessionService: IWorkbenchSessionService) {
    super(options);
    this.sessionService = sessionService;
    this.contentElement.classList.add("zeta-chat-agent-sidebar-view");
    this.own(sessionService.onDidChange(() => this.render()));
    this.render();
  }

  override focus(): void {
    this.contentElement.querySelector<HTMLButtonElement>("button")?.focus();
  }

  private render(): void {
    const sessions = this.sessionService.sessions.filter((session) => session.status !== "archived");
    const active = this.sessionService.active;
    const content = this.contentElement;
    content.replaceChildren();
    if (sessions.length === 0) {
      const empty = content.ownerDocument.createElement("p");
      empty.className = "zeta-chat-agent-sidebar-empty";
      empty.textContent = "No agent sessions yet.";
      content.append(empty);
      return;
    }
    for (const session of sessions) content.append(this.createSessionButton(session, active?.session.sessionId, active?.threadId));
  }

  private createSessionButton(session: Session, activeSessionId: string | undefined, activeThreadId: string | undefined): HTMLButtonElement {
    const button = this.contentElement.ownerDocument.createElement("button");
    button.className = "zeta-chat-agent-session";
    button.type = "button";
    const selected = session.sessionId === activeSessionId;
    button.classList.toggle("checked", selected);
    button.setAttribute("aria-pressed", String(selected));
    const title = this.contentElement.ownerDocument.createElement("span");
    title.className = "zeta-chat-agent-session-title";
    title.textContent = session.title;
    const detail = this.contentElement.ownerDocument.createElement("span");
    detail.className = "zeta-chat-agent-session-detail";
    detail.textContent = sessionDetail(session, activeSessionId === session.sessionId ? activeThreadId : undefined);
    button.append(title, detail);
    const thread = activeThread(session);
    button.disabled = thread === undefined;
    button.addEventListener("click", () => {
      if (thread) this.sessionService.selectThread(session.sessionId, thread.threadId);
    });
    return button;
  }
}

function activeThread(session: Session): SessionThread | undefined {
  return session.threads.find((thread) => thread.status === "active");
}

function sessionDetail(session: Session, selectedThreadId: string | undefined): string {
  const thread = selectedThreadId === undefined
    ? activeThread(session)
    : session.threads.find((candidate) => candidate.threadId === selectedThreadId);
  if (!thread) return session.status;
  return thread.origin.type === "root" ? "Active agent" : "Agent branch";
}
