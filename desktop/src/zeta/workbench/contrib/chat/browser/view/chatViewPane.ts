import "../media/chat.css";
import type { Session, SessionId, SessionThread, ThreadId } from "../../../../../../../generated/app-server/types.js";
import { setDisposableOwner } from "../../../../../base/common/lifecycle.js";
import type { ZetaRendererApi } from "../../../../../platform/app-server/common/renderer-api.js";
import type { IMenuService } from "../../../../../platform/actions/common/menuService.js";
import type { IContextKeyService } from "../../../../../platform/contextkey/common/contextkey.js";
import type { IContextMenuService } from "../../../../../platform/contextview/browser/contextMenu.js";
import { SidebarPart } from "../../../../browser/parts/sidebar/sidebarPart.js";
import { ViewPane, type IViewPaneOptions } from "../../../../browser/parts/views/viewPane.js";
import { ViewContainerLocation } from "../../../../common/views.js";
import type { IActiveSessionThread, IWorkbenchSessionService } from "../../../../services/sessions/common/sessionService.js";
import type { IViewDescriptorService } from "../../../../services/views/common/viewDescriptorService.js";
import { AgentSidebarVisibleContext } from "../../common/chat.js";
import { ChatPane } from "../pane/chatPane.js";
import { ChatTitleControl } from "./chatTitleControl.js";

let chatViewInstanceId = 0;
let chatPaneInstanceId = 0;

/**
 * Session-level Chat container.
 *
 * Each active Session owns one retained ChatPane. Thread selection remains
 * internal to that Pane, while the title tabs only switch Sessions.
 */
export class ChatViewPane extends ViewPane {
  readonly #api: ZetaRendererApi;
  readonly #sessionService: IWorkbenchSessionService;
  readonly #contextMenuService: IContextMenuService;
  readonly #titleControl: ChatTitleControl;
  readonly #agentSidebar: SidebarPart;
  readonly #paneHost: HTMLDivElement;
  readonly #empty: HTMLDivElement;
  readonly #panes = new Map<SessionId, ChatPane>();
  #activePane: ChatPane | undefined;

  constructor(
    options: IViewPaneOptions,
    api: ZetaRendererApi,
    sessionService: IWorkbenchSessionService,
    menuService: IMenuService,
    contextMenuService: IContextMenuService,
    viewDescriptorService: IViewDescriptorService,
    contextKeyService: IContextKeyService,
  ) {
    super(options);
    this.#api = api;
    this.#sessionService = sessionService;
    this.#contextMenuService = contextMenuService;
    this.element.classList.add("zeta-chat-view-pane");
    this.titleElement.hidden = true;
    this.contentElement.classList.add("zeta-chat-view");
    const viewId = `zeta-chat-view-${++chatViewInstanceId}`;
    this.#titleControl = this.own(new ChatTitleControl(
      options.ownerDocument,
      viewId,
      {
        selectSession: (sessionId) => this.#selectSession(sessionId),
        closeSession: (sessionId) => this.#closeSession(sessionId),
      },
      menuService,
      contextMenuService,
    ));
    this.#agentSidebar = this.own(new SidebarPart({
      ownerDocument: options.ownerDocument,
      viewDescriptorService,
      id: "agentSidebar",
      location: ViewContainerLocation.AgentSidebar,
      ariaLabel: "Agent sidebar",
      viewsAriaLabel: "Agent sidebar views",
    }));
    this.#agentSidebar.element.classList.add("zeta-chat-agent-sidebar");
    const agentSidebarVisible = AgentSidebarVisibleContext.bindTo(contextKeyService);
    this.defer(() => agentSidebarVisible.reset());
    this.#paneHost = options.ownerDocument.createElement("div");
    this.#paneHost.className = "zeta-chat-pane-host";
    this.#empty = options.ownerDocument.createElement("div");
    this.#empty.className = "zeta-chat-empty zeta-chat-view-empty";
    this.#empty.textContent = "Start a new chat to begin.";
    const body = options.ownerDocument.createElement("div");
    body.className = "zeta-chat-body";
    body.append(this.#paneHost, this.#agentSidebar.element);
    this.contentElement.append(this.#titleControl.element, body);
    this.own(sessionService.onDidChange(() => this.#syncSessions()));
    this.own(contextKeyService.onDidChangeContext((event) => {
      if (event.keys.has(AgentSidebarVisibleContext.key)) {
        this.#syncAgentSidebarVisibility(contextKeyService);
      }
    }));
    this.defer(() => {
      for (const pane of this.#panes.values()) pane.dispose();
      this.#panes.clear();
    });
    this.#syncAgentSidebarVisibility(contextKeyService);
    this.#syncSessions();
    void sessionService.initialize();
  }

  override focus(): void {
    this.#activePane?.focus();
  }

  #syncSessions(): void {
    const entries: { readonly session: Session; readonly pane: ChatPane }[] = [];
    const retainedSessionIds = new Set<SessionId>();
    for (const session of this.#sessionService.sessions) {
      if (session.status !== "active") continue;
      const selection = this.#selectionForSession(session);
      if (!selection) continue;
      retainedSessionIds.add(session.sessionId);
      let pane = this.#panes.get(session.sessionId);
      if (!pane) {
        pane = new ChatPane(
          this.element.ownerDocument,
          `zeta-chat-pane-${++chatPaneInstanceId}`,
          this.#api,
          selection,
          this.#sessionService,
          this.#contextMenuService,
        );
        setDisposableOwner(pane, this);
        this.#panes.set(session.sessionId, pane);
      } else {
        void pane.selectThread(selection);
      }
      entries.push({ session, pane });
    }
    for (const [sessionId, pane] of this.#panes) {
      if (retainedSessionIds.has(sessionId)) continue;
      this.#panes.delete(sessionId);
      pane.dispose();
    }
    this.#paneHost.replaceChildren(...entries.map(({ pane }) => pane.element), this.#empty);
    const activeSessionId = this.#sessionService.active?.session.sessionId;
    this.#activePane = activeSessionId ? this.#panes.get(activeSessionId) : undefined;
    for (const { pane } of entries) pane.setVisible(pane === this.#activePane);
    this.#empty.hidden = entries.length > 0;
    const tabIds = this.#titleControl.setSessions(
      entries.map(({ session, pane }) => ({ session, panelId: pane.element.id })),
      this.#activePane?.sessionId,
    );
    for (const { pane } of entries) pane.setTabId(tabIds.get(pane.sessionId));
  }

  #selectionForSession(session: Session): IActiveSessionThread | undefined {
    const active = this.#sessionService.active;
    if (
      active?.session.sessionId === session.sessionId &&
      isActiveThread(session, active.threadId)
    ) {
      return { session, threadId: active.threadId };
    }
    const retainedThreadId = this.#panes.get(session.sessionId)?.threadId;
    if (retainedThreadId && isActiveThread(session, retainedThreadId)) {
      return { session, threadId: retainedThreadId };
    }
    const thread = rootThread(session) ?? session.threads.find((candidate) => candidate.status === "active");
    return thread ? { session, threadId: thread.threadId } : undefined;
  }

  #selectSession(sessionId: SessionId): void {
    const pane = this.#panes.get(sessionId);
    if (!pane) return;
    this.#sessionService.selectThread(sessionId, pane.threadId);
  }

  #closeSession(sessionId: SessionId): void {
    void this.#sessionService.archiveSession(sessionId).catch(() => {});
  }

  #syncAgentSidebarVisibility(contextKeyService: IContextKeyService): void {
    const visible = contextKeyService.getValue<boolean>(AgentSidebarVisibleContext.key) ?? AgentSidebarVisibleContext.defaultValue;
    this.#agentSidebar.setVisible(visible);
  }
}

function isActiveThread(session: Session, threadId: ThreadId): boolean {
  return session.threads.some((thread) => thread.threadId === threadId && thread.status === "active");
}

function rootThread(session: Session): SessionThread | undefined {
  return session.threads.find((thread) => thread.status === "active" && thread.origin.type === "root");
}
