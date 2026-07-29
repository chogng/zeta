import { TabList } from "../../../../../base/browser/ui/tablist/tabList.js";
import { DisposableOwner } from "../../../../../base/common/lifecycle.js";
import type { Session, SessionId } from "../../../../../../../generated/app-server/types.js";

export interface ChatSessionTab {
  readonly session: Session;
  readonly panelId: string;
}

interface ChatTabDescriptor {
  readonly sessionId: SessionId;
  readonly label: string;
  readonly panelId: string;
  readonly tabId: string;
}

/** Callback through which a Chat tab requests Session selection. */
export interface ChatTabsDelegate {
  selectSession(sessionId: SessionId): void;
}

/** Maps active Sessions onto the shared TabList. */
export class ChatTabsControl extends DisposableOwner {
  readonly element: HTMLDivElement;
  readonly #tabList: TabList<SessionId>;
  readonly #idPrefix: string;
  readonly #tabIds = new Map<string, string>();
  #nextTabId = 0;

  constructor(ownerDocument: Document, idPrefix: string, delegate: ChatTabsDelegate) {
    super();
    this.#idPrefix = idPrefix;
    this.element = ownerDocument.createElement("div");
    this.element.className = "zeta-chat-tabs-control";
    this.#tabList = this.own(new TabList({
      ownerDocument,
      ariaLabel: "Open chats",
      onActivate: (sessionId) => delegate.selectSession(sessionId),
    }));
    this.element.append(this.#tabList.element);
    this.defer(() => this.element.remove());
  }

  setSessions(entries: readonly ChatSessionTab[], activeSessionId: SessionId | undefined): ReadonlyMap<SessionId, string> {
    const tabs = entries.map(({ session, panelId }) => {
      let tabId = this.#tabIds.get(session.sessionId);
      if (!tabId) {
        tabId = `${this.#idPrefix}-tab-${++this.#nextTabId}`;
        this.#tabIds.set(session.sessionId, tabId);
      }
      return {
        sessionId: session.sessionId,
        label: session.title.trim() || "Chat",
        panelId,
        tabId,
      } satisfies ChatTabDescriptor;
    });
    this.#tabList.setTabs(
      tabs.map((tab) => ({
        id: tab.sessionId,
        value: tab.sessionId,
        label: tab.label,
        tabId: tab.tabId,
        panelId: tab.panelId,
      })),
      activeSessionId,
    );
    this.element.hidden = tabs.length === 0;
    return new Map(tabs.map((tab) => [tab.sessionId, tab.tabId]));
  }
}
