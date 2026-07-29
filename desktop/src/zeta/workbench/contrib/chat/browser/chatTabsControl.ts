import { TabList } from "../../../../base/browser/ui/tablist/tabList.js";
import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import type {
  Session,
  SessionId,
  ThreadId,
} from "../../../../../../generated/app-server/types.js";
import type { IActiveSessionThread } from "../../../services/sessions/common/sessionService.js";

interface ChatTabIdentity {
  readonly sessionId: SessionId;
  readonly threadId: ThreadId;
}

interface ChatTabDescriptor extends ChatTabIdentity {
  readonly label: string;
  readonly tabId: string;
}

/** Callback through which a Chat tab requests Thread selection. */
export interface ChatTabsDelegate {
  selectThread(sessionId: SessionId, threadId: ThreadId): void;
}

/** Maps active Session Threads onto the shared TabList. */
export class ChatTabsControl extends DisposableOwner {
  readonly element: HTMLDivElement;
  readonly #tabList: TabList<ChatTabIdentity>;
  readonly #panelId: string;
  readonly #tabIds = new Map<string, string>();
  #nextTabId = 0;

  constructor(
    ownerDocument: Document,
    panelId: string,
    delegate: ChatTabsDelegate,
  ) {
    super();
    this.#panelId = panelId;
    this.element = ownerDocument.createElement("div");
    this.element.className = "zeta-chat-tabs-control";
    this.#tabList = this.own(new TabList({
      ownerDocument,
      ariaLabel: "Open chats",
      onActivate: ({ sessionId, threadId }) => {
        delegate.selectThread(sessionId, threadId);
      },
    }));
    this.element.append(this.#tabList.element);
    this.defer(() => this.element.remove());
  }

  setSessions(
    sessions: readonly Session[],
    active: IActiveSessionThread | undefined,
  ): string | undefined {
    const tabs = sessions.flatMap((session) => {
      if (session.status !== "active") return [];
      const activeThreads = session.threads.filter(
        (thread) => thread.status === "active",
      );
      return activeThreads.map((thread, index) => {
        const key = chatTabKey(session.sessionId, thread.threadId);
        let tabId = this.#tabIds.get(key);
        if (!tabId) {
          tabId = `${this.#panelId}-tab-${++this.#nextTabId}`;
          this.#tabIds.set(key, tabId);
        }
        return {
          sessionId: session.sessionId,
          threadId: thread.threadId,
          label: chatTabLabel(session, index, activeThreads.length),
          tabId,
        } satisfies ChatTabDescriptor;
      });
    });
    const activeKey = active
      ? chatTabKey(active.session.sessionId, active.threadId)
      : undefined;
    this.#tabList.setTabs(
      tabs.map((tab) => ({
        id: chatTabKey(tab.sessionId, tab.threadId),
        value: {
          sessionId: tab.sessionId,
          threadId: tab.threadId,
        },
        label: tab.label,
        tabId: tab.tabId,
        panelId: this.#panelId,
      })),
      activeKey,
    );
    this.element.hidden = tabs.length === 0;
    return tabs.find(
      (tab) => chatTabKey(tab.sessionId, tab.threadId) === activeKey,
    )?.tabId;
  }
}

function chatTabKey(sessionId: SessionId, threadId: ThreadId): string {
  return JSON.stringify([sessionId, threadId]);
}

function chatTabLabel(
  session: Session,
  threadIndex: number,
  threadCount: number,
): string {
  const title = session.title.trim() || "Chat";
  return threadCount > 1 ? `${title} ${threadIndex + 1}` : title;
}
