import "./chatTitleControl.css";
import { DisposableOwner } from "../../../../../base/common/lifecycle.js";
import { AnchorPosition } from "../../../../../base/common/layout.js";
import { MenuWorkbenchToolBar } from "../../../../../platform/actions/browser/toolbar.js";
import { MenuId } from "../../../../../platform/actions/common/actions.js";
import type { IMenuService } from "../../../../../platform/actions/common/menuService.js";
import type { IContextMenuService } from "../../../../../platform/contextview/browser/contextMenu.js";
import { ChatTabsControl, type ChatTab, type ChatTabsDelegate } from "./chatTabsControl.js";
import { MultiChatTabsControl } from "./multiChatTabsControl.js";
import type { PartTitleProjection } from "../../../../browser/parts/views/viewPane.js";
import { h } from "../../../../../base/browser/dom.js";

/** Owns Chat's title content and action projections. */
export class ChatTitleControl extends DisposableOwner {
  private readonly tabs: ChatTabsControl;
  private readonly actionsElement: HTMLDivElement;

  constructor(ownerDocument: Document, idPrefix: string, delegate: ChatTabsDelegate, menuService: IMenuService, contextMenuService: IContextMenuService) {
    super();
    this.tabs = this.own(new MultiChatTabsControl(ownerDocument, idPrefix, delegate, "pane-title"));
    const toolbar = this.own(new MenuWorkbenchToolBar(
      menuService,
      contextMenuService,
      MenuId.ChatTitle,
      ownerDocument,
      { hoverAnchorPosition: AnchorPosition.Below },
    ));
    toolbar.element.setAttribute("aria-label", "Chat actions");
    const layoutToolbar = this.own(new MenuWorkbenchToolBar(
      menuService,
      contextMenuService,
      MenuId.ChatTitleLayout,
      ownerDocument,
      { highlightToggledItems: true, hoverAnchorPosition: AnchorPosition.Below },
    ));
    layoutToolbar.element.setAttribute("aria-label", "Chat layout");
    layoutToolbar.element.classList.add("zeta-chat-title-layout-actions");
    this.actionsElement = h(ownerDocument, "div");
    this.actionsElement.className = "zeta-chat-title-actions";
    this.actionsElement.append(toolbar.element, layoutToolbar.element);
    this.defer(() => this.actionsElement.remove());
  }

  get partTitleProjection(): PartTitleProjection {
    return { content: this.tabs.element, actions: this.actionsElement };
  }

  setTabs(entries: readonly ChatTab[], activeTabId: string | undefined): ReadonlyMap<string, string> {
    return this.tabs.setTabs(entries, activeTabId);
  }
}
