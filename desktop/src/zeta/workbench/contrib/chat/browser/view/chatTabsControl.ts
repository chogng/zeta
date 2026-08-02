import "./chatTabsControl.css";
import { TabList } from "../../../../../base/browser/ui/tablist/tabList.js";
import { DisposableOwner } from "../../../../../base/common/lifecycle.js";

export interface ChatTab {
  readonly id: string;
  readonly label: string;
  readonly panelId: string;
}

export type ChatTabsPresentation = "pane-title";

interface ChatTabDescriptor {
  readonly id: string;
  readonly label: string;
  readonly panelId: string;
  readonly tabId: string;
}

/** Callback through which a Chat tab requests Session selection. */
export interface ChatTabsDelegate {
  selectTab(tabId: string): void;
  closeTab(tabId: string): void;
}

/** Maps untitled and durable Chat sessions onto the shared TabList. */
export class ChatTabsControl extends DisposableOwner {
  readonly element: HTMLDivElement;
  private readonly tabList: TabList<string>;
  private readonly idPrefix: string;
  private readonly tabIds = new Map<string, string>();
  private nextTabId = 0;

  constructor(ownerDocument: Document, idPrefix: string, delegate: ChatTabsDelegate, presentation: ChatTabsPresentation) {
    super();
    this.idPrefix = idPrefix;
    this.element = ownerDocument.createElement("div");
    this.element.className = "zeta-chat-tabs-control";
    this.element.classList.add(`zeta-chat-tabs-${presentation}`);
    this.tabList = this.own(new TabList({
      ownerDocument,
      ariaLabel: "Open chats",
      onActivate: (tabId) => delegate.selectTab(tabId),
      onClose: (tabId) => delegate.closeTab(tabId),
    }));
    this.element.append(this.tabList.element);
    this.defer(() => this.element.remove());
  }

  setTabs(entries: readonly ChatTab[], activeTabId: string | undefined): ReadonlyMap<string, string> {
    const tabs = entries.map(({ id, label, panelId }) => {
      let tabId = this.tabIds.get(id);
      if (!tabId) {
        tabId = `${this.idPrefix}-tab-${++this.nextTabId}`;
        this.tabIds.set(id, tabId);
      }
      return {
        id,
        label,
        panelId,
        tabId,
      } satisfies ChatTabDescriptor;
    });
    this.tabList.setTabs(
      tabs.map((tab) => ({
        id: tab.id,
        value: tab.id,
        label: tab.label,
        tabId: tab.tabId,
        panelId: tab.panelId,
      })),
      activeTabId,
    );
    this.element.hidden = tabs.length === 0;
    return new Map(tabs.map((tab) => [tab.id, tab.tabId]));
  }
}
