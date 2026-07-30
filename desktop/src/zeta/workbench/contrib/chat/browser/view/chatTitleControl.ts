import "./chatTitleControl.css";
import { DisposableOwner } from "../../../../../base/common/lifecycle.js";
import { MenuWorkbenchToolBar } from "../../../../../platform/actions/browser/toolbar.js";
import { MenuId } from "../../../../../platform/actions/common/actions.js";
import type { IMenuService } from "../../../../../platform/actions/common/menuService.js";
import type { IContextMenuService } from "../../../../../platform/contextview/browser/contextMenu.js";
import type { SessionId } from "../../../../../../../generated/app-server/types.js";
import { ChatTabsControl, type ChatSessionTab, type ChatTabsDelegate } from "./chatTabsControl.js";

/** Hosts Chat tabs and the independent Chat action toolbar. */
export class ChatTitleControl extends DisposableOwner {
  static readonly HEIGHT = 35;

  readonly element: HTMLDivElement;
  private readonly tabs: ChatTabsControl;

  constructor(ownerDocument: Document, idPrefix: string, delegate: ChatTabsDelegate, menuService: IMenuService, contextMenuService: IContextMenuService) {
    super();
    this.element = ownerDocument.createElement("div");
    this.element.className = "zeta-chat-title-control";
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
    const actions = ownerDocument.createElement("div");
    actions.className = "zeta-chat-title-actions";
    actions.append(toolbar.element, layoutToolbar.element);
    this.element.append(this.tabs.element, actions);
    this.defer(() => this.element.remove());
  }

  setSessions(entries: readonly ChatSessionTab[], activeSessionId: SessionId | undefined): ReadonlyMap<SessionId, string> {
    return this.tabs.setSessions(entries, activeSessionId);
  }
}
