import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import { MenuWorkbenchToolBar } from "../../../../platform/actions/browser/toolbar.js";
import { MenuId } from "../../../../platform/actions/common/actions.js";
import type { IMenuService } from "../../../../platform/actions/common/menuService.js";
import type { IContextMenuService } from "../../../../platform/contextview/browser/contextMenu.js";
import type { IWorkbenchSessionService } from "../../../services/sessions/common/sessionService.js";
import { ChatTabsControl } from "./chatTabsControl.js";

/** Hosts Chat tabs and the independent Chat action toolbar. */
export class ChatTitleControl extends DisposableOwner {
  static readonly HEIGHT = 35;

  readonly element: HTMLDivElement;
  readonly #tabs: ChatTabsControl;
  readonly #sessionService: IWorkbenchSessionService;

  constructor(
    ownerDocument: Document,
    panelId: string,
    sessionService: IWorkbenchSessionService,
    menuService: IMenuService,
    contextMenuService: IContextMenuService,
  ) {
    super();
    this.#sessionService = sessionService;
    this.element = ownerDocument.createElement("div");
    this.element.className = "zeta-chat-title-control";
    this.#tabs = this.own(new ChatTabsControl(
      ownerDocument,
      panelId,
      {
        selectThread: (sessionId, threadId) => {
          sessionService.selectThread(sessionId, threadId);
        },
      },
    ));
    const toolbar = this.own(new MenuWorkbenchToolBar(
      menuService,
      contextMenuService,
      MenuId.ChatTitle,
      ownerDocument,
    ));
    toolbar.element.setAttribute("aria-label", "Chat actions");
    const actions = ownerDocument.createElement("div");
    actions.className = "zeta-chat-title-actions";
    actions.append(toolbar.element);
    this.element.append(this.#tabs.element, actions);
    this.defer(() => this.element.remove());
  }

  refresh(): string | undefined {
    return this.#tabs.setSessions(
      this.#sessionService.sessions,
      this.#sessionService.active,
    );
  }
}
