import "../media/chat.css";
import { setDisposableOwner } from "../../../../../base/common/lifecycle.js";
import type { IMenuService } from "../../../../../platform/actions/common/menuService.js";
import type { IContextMenuService } from "../../../../../platform/contextview/browser/contextMenu.js";
import type { ICommandService } from "../../../../../platform/commands/common/commands.js";
import { ViewPane, type IViewPaneOptions, type PartTitleProjection } from "../../../../browser/parts/views/viewPane.js";
import type { IWorkbenchLayoutService } from "../../../../services/layout/browser/layoutService.js";
import type { IChatService } from "../../../../services/chat/common/chatService.js";
import type { IActiveSessionThread, IUntitledChatSession, IWorkbenchSessionService, Session, SessionThread, ThreadId } from "../../../../services/sessions/common/sessionService.js";
import { ChatPane } from "../pane/chatPane.js";
import { ChatTitleControl } from "./chatTitleControl.js";

let chatViewInstanceId = 0;
let chatPaneInstanceId = 0;

interface ChatPaneEntry {
  readonly tabId: string;
  readonly label: string;
  readonly pane: ChatPane;
}

/**
 * Chat tab container.
 *
 * Each untitled or active durable Session owns one retained ChatPane. Thread
 * selection remains internal to durable panes, while untitled sessions
 * materialize only when their first message is sent.
 */
export class ChatViewPane extends ViewPane {
  private readonly chatService: IChatService;
  private readonly sessionService: IWorkbenchSessionService;
  private readonly contextMenuService: IContextMenuService;
  private readonly commandService: ICommandService;
  private readonly titleControl: ChatTitleControl;
  private readonly paneHost: HTMLDivElement;
  private readonly empty: HTMLDivElement;
  private readonly panes = new Map<string, ChatPane>();
  private activePane: ChatPane | undefined;
  private viewDisposed = false;

  constructor(
    options: IViewPaneOptions,
    chatService: IChatService,
    sessionService: IWorkbenchSessionService,
    menuService: IMenuService,
    contextMenuService: IContextMenuService,
    commandService: ICommandService,
    private readonly layoutService: IWorkbenchLayoutService,
  ) {
    super(options);
    this.chatService = chatService;
    this.sessionService = sessionService;
    this.contextMenuService = contextMenuService;
    this.commandService = commandService;
    this.element.classList.add("zeta-chat-view-pane");
    this.titleElement.hidden = true;
    this.contentElement.classList.add("zeta-chat-view");
    const viewId = `zeta-chat-view-${++chatViewInstanceId}`;
    this.titleControl = this.own(new ChatTitleControl(
      options.ownerDocument,
      viewId,
      {
        selectTab: (tabId) => this.selectTab(tabId),
        closeTab: (tabId) => this.closeTab(tabId),
      },
      menuService,
      contextMenuService,
    ));
    this.paneHost = options.ownerDocument.createElement("div");
    this.paneHost.className = "zeta-chat-pane-host";
    this.empty = options.ownerDocument.createElement("div");
    this.empty.className = "zeta-chat-empty zeta-chat-view-empty";
    this.empty.textContent = "Start a new chat to begin.";
    const body = options.ownerDocument.createElement("div");
    body.className = "zeta-chat-body";
    body.append(this.paneHost);
    this.contentElement.append(body);
    this.own(sessionService.onDidChange(() => this.syncSessions()));
    this.own(layoutService.onDidChangePartVisibility((event) => {
      if (event.partId === "auxiliarybar" && event.visible) this.ensureTabForVisibleChat();
    }));
    this.defer(() => {
      this.viewDisposed = true;
      for (const pane of this.panes.values()) pane.dispose();
      this.panes.clear();
    });
    this.syncSessions();
    this.ensureTabForVisibleChat();
    void sessionService.initialize().then(() => {
      if (this.viewDisposed) return;
      this.ensureTabForVisibleChat();
    });
  }

  override focus(): void {
    this.activePane?.focus();
  }

  override get partTitleProjection(): PartTitleProjection {
    return this.titleControl.partTitleProjection;
  }

  private syncSessions(): void {
    this.rekeyMaterializedPanes();
    const entries: ChatPaneEntry[] = [];
    const retainedPaneIds = new Set<string>();
    for (const untitledSession of this.sessionService.untitledSessions) {
      const paneId = untitledSessionPaneId(untitledSession);
      retainedPaneIds.add(paneId);
      let pane = this.panes.get(paneId);
      if (!pane) {
        pane = new ChatPane(
          this.element.ownerDocument,
          `zeta-chat-pane-${++chatPaneInstanceId}`,
          this.chatService,
          { kind: "untitled", session: untitledSession },
          this.sessionService,
          this.contextMenuService,
          this.commandService,
        );
        setDisposableOwner(pane, this);
        this.panes.set(paneId, pane);
      } else {
        pane.selectUntitledSession(untitledSession);
      }
      entries.push({ tabId: pane.element.id, label: untitledSession.title.trim() || "New Chat", pane });
    }
    for (const session of this.sessionService.sessions) {
      if (session.status !== "active") continue;
      const selection = this.selectionForSession(session);
      if (!selection) continue;
      const paneId = sessionPaneId(session);
      retainedPaneIds.add(paneId);
      let pane = this.panes.get(paneId);
      if (!pane) {
        pane = new ChatPane(
          this.element.ownerDocument,
          `zeta-chat-pane-${++chatPaneInstanceId}`,
          this.chatService,
          { kind: "session", active: selection },
          this.sessionService,
          this.contextMenuService,
          this.commandService,
        );
        setDisposableOwner(pane, this);
        this.panes.set(paneId, pane);
      } else {
        void pane.selectThread(selection);
      }
      entries.push({ tabId: pane.element.id, label: session.title.trim() || "Chat", pane });
    }
    for (const [paneId, pane] of this.panes) {
      if (retainedPaneIds.has(paneId)) continue;
      this.panes.delete(paneId);
      pane.dispose();
    }
    this.paneHost.replaceChildren(...entries.map((entry) => entry.pane.element), this.empty);
    const activePaneId = this.activePaneId();
    this.activePane = activePaneId ? this.panes.get(activePaneId) : undefined;
    for (const entry of entries) entry.pane.setVisible(entry.pane === this.activePane);
    this.empty.hidden = entries.length > 0;
    const activeTabId = this.activePane?.element.id;
    const tabIds = this.titleControl.setTabs(
      entries.map((entry) => ({ id: entry.tabId, label: entry.label, panelId: entry.pane.element.id })),
      activeTabId,
    );
    for (const entry of entries) entry.pane.setTabId(tabIds.get(entry.tabId));
  }

  private selectionForSession(session: Session): IActiveSessionThread | undefined {
    const active = this.sessionService.active;
    if (
      active?.session.sessionId === session.sessionId &&
      isActiveThread(session, active.threadId)
    ) {
      return { session, threadId: active.threadId };
    }
    const retainedThreadId = this.panes.get(sessionPaneId(session))?.threadId;
    if (retainedThreadId && isActiveThread(session, retainedThreadId)) {
      return { session, threadId: retainedThreadId };
    }
    const thread = rootThread(session) ?? session.threads.find((candidate) => candidate.status === "active");
    return thread ? { session, threadId: thread.threadId } : undefined;
  }

  private selectTab(tabId: string): void {
    const pane = this.paneForTabId(tabId);
    if (!pane) return;
    const untitledSessionId = pane.untitledSessionId;
    if (untitledSessionId) {
      this.sessionService.selectUntitledSession(untitledSessionId);
      return;
    }
    const sessionId = pane.sessionId;
    const threadId = pane.threadId;
    if (sessionId && threadId) this.sessionService.selectThread(sessionId, threadId);
  }

  private closeTab(tabId: string): void {
    const pane = this.paneForTabId(tabId);
    if (!pane) return;
    const untitledSessionId = pane.untitledSessionId;
    if (untitledSessionId) {
      this.sessionService.discardUntitledSession(untitledSessionId);
      this.hideChatWhenEmpty();
      return;
    }
    const sessionId = pane.sessionId;
    if (!sessionId) return;
    void this.sessionService.archiveSession(sessionId).then(() => this.hideChatWhenEmpty()).catch(() => {});
  }

  private ensureTabForVisibleChat(): void {
    if (!this.layoutService.isPartVisible("auxiliarybar") || this.panes.size > 0) return;
    this.sessionService.createUntitledSession();
  }

  private hideChatWhenEmpty(): void {
    if (this.panes.size === 0) this.layoutService.hidePart("auxiliarybar");
  }

  private activePaneId(): string | undefined {
    const untitledSession = this.sessionService.activeUntitledSession;
    if (untitledSession) return untitledSessionPaneId(untitledSession);
    const active = this.sessionService.active;
    return active ? sessionPaneId(active.session) : undefined;
  }

  private rekeyMaterializedPanes(): void {
    for (const [paneId, pane] of [...this.panes]) {
      const sessionId = pane.sessionId;
      if (!sessionId) continue;
      const materializedPaneId = sessionPaneIdFromId(sessionId);
      if (paneId === materializedPaneId) continue;
      const existing = this.panes.get(materializedPaneId);
      if (existing && existing !== pane) {
        pane.dispose();
        this.panes.delete(paneId);
        continue;
      }
      this.panes.delete(paneId);
      this.panes.set(materializedPaneId, pane);
    }
  }

  private paneForTabId(tabId: string): ChatPane | undefined {
    return [...this.panes.values()].find((pane) => pane.element.id === tabId);
  }

}

function isActiveThread(session: Session, threadId: ThreadId): boolean {
  return session.threads.some((thread) => thread.threadId === threadId && thread.status === "active");
}

function rootThread(session: Session): SessionThread | undefined {
  return session.threads.find((thread) => thread.status === "active" && thread.origin.type === "root");
}

function untitledSessionPaneId(session: IUntitledChatSession): string {
  return `untitled:${session.untitledSessionId}`;
}

function sessionPaneId(session: Session): string {
  return sessionPaneIdFromId(session.sessionId);
}

function sessionPaneIdFromId(sessionId: string): string {
  return `session:${sessionId}`;
}
