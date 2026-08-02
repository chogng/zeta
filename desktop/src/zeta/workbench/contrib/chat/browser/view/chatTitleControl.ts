import "./chatTitleControl.css";
import { DisposableOwner } from "../../../../../base/common/lifecycle.js";
import { MenuWorkbenchToolBar } from "../../../../../platform/actions/browser/toolbar.js";
import { MenuId } from "../../../../../platform/actions/common/actions.js";
import type { IMenuService } from "../../../../../platform/actions/common/menuService.js";
import type { IContextMenuService } from "../../../../../platform/contextview/browser/contextMenu.js";
import { ChatTabsControl, type ChatTab, type ChatTabsDelegate } from "./chatTabsControl.js";

/** Owns Chat's title content and action projections. */
export class ChatTitleControl extends DisposableOwner {
  static readonly HEIGHT = 35;

  private readonly tabs: ChatTabsControl;
  private readonly actionsElement: HTMLDivElement;

  constructor(ownerDocument: Document, idPrefix: string, delegate: ChatTabsDelegate, menuService: IMenuService, contextMenuService: IContextMenuService) {
    super();
    this.tabs = this.own(new ChatTabsControl(ownerDocument, idPrefix, delegate));
    const toolbar = this.own(new MenuWorkbenchToolBar(
      menuService,
      contextMenuService,
      MenuId.ChatTitle,
      ownerDocument,
    ));
    toolbar.element.setAttribute("aria-label", "Chat actions");
    const layoutToolbar = this.own(new MenuWorkbenchToolBar(
      menuService,
      contextMenuService,
      MenuId.ChatTitleLayout,
      ownerDocument,
      { highlightToggledItems: true },
    ));
    layoutToolbar.element.setAttribute("aria-label", "Chat layout");
    layoutToolbar.element.classList.add("zeta-chat-title-layout-actions");
    this.actionsElement = ownerDocument.createElement("div");
    this.actionsElement.className = "zeta-chat-title-actions";
    this.actionsElement.append(toolbar.element, layoutToolbar.element);
    this.defer(() => this.actionsElement.remove());
  }

  get partTitleElement(): HTMLElement {
    return this.tabs.element;
  }

  get partTitleActionsElement(): HTMLElement {
    return this.actionsElement;
  }

  setTabs(entries: readonly ChatTab[], activeTabId: string | undefined): ReadonlyMap<string, string> {
    return this.tabs.setTabs(entries, activeTabId);
  }
}
