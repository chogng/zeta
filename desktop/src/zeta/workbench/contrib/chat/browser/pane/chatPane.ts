import type { SessionId, ThreadId } from "../../../../../../../generated/app-server/types.js";
import { DisposableOwner } from "../../../../../base/common/lifecycle.js";
import type { ZetaRendererApi } from "../../../../../platform/app-server/common/renderer-api.js";
import type { IContextMenuService } from "../../../../../platform/contextview/browser/contextMenu.js";
import type { IActiveSessionThread, IWorkbenchSessionService } from "../../../../services/sessions/common/sessionService.js";
import type { ChatInputDelegate } from "../input/chatInput.js";
import { ChatInputWidget } from "../input/chatInputWidget.js";
import { ChatListWidget } from "../list/chatListWidget.js";
import { ChatPaneModel } from "./chatPaneModel.js";

/** Owns the content, interaction state, and Thread projection for one Session. */
export class ChatPane extends DisposableOwner {
  readonly element: HTMLElement;
  readonly sessionId: SessionId;
  readonly #model: ChatPaneModel;
  readonly #listWidget: ChatListWidget;
  readonly #inputWidget: ChatInputWidget;

  constructor(ownerDocument: Document, panelId: string, api: ZetaRendererApi, active: IActiveSessionThread, sessionService: IWorkbenchSessionService, contextMenuService: IContextMenuService) {
    super();
    this.sessionId = active.session.sessionId;
    this.element = ownerDocument.createElement("div");
    this.element.id = panelId;
    this.element.className = "zeta-chat";
    this.element.dataset.sessionId = this.sessionId;
    this.element.setAttribute("role", "tabpanel");
    this.element.hidden = true;
    this.#model = this.own(new ChatPaneModel(api, active, sessionService));
    this.#listWidget = this.own(new ChatListWidget(ownerDocument));
    const inputDelegate: ChatInputDelegate = {
      send: (text) => this.#model.send(text),
      interrupt: () => this.#model.interrupt(),
      selectModel: (model) => this.#model.selectModel(model),
      resolveInteraction: (response) => this.#model.resolveInteraction(response),
    };
    this.#inputWidget = this.own(new ChatInputWidget(ownerDocument, inputDelegate, contextMenuService));
    this.element.append(this.#inputWidget.element, this.#listWidget.element);
    this.own(this.#model.onDidChange(() => this.#render()));
    this.defer(() => this.element.remove());
    this.#render();
  }

  get threadId(): ThreadId {
    return this.#model.threadId;
  }

  selectThread(active: IActiveSessionThread): Promise<void> {
    if (active.session.sessionId !== this.sessionId) {
      throw new Error(`ChatPane cannot select a Thread from another Session: ${active.session.sessionId}`);
    }
    this.element.dataset.threadId = active.threadId;
    return this.#model.selectThread(active);
  }

  setTabId(tabId: string | undefined): void {
    if (tabId) {
      this.element.setAttribute("aria-labelledby", tabId);
      this.element.removeAttribute("aria-label");
    } else {
      this.element.removeAttribute("aria-labelledby");
      this.element.setAttribute("aria-label", "Chat");
    }
  }

  setVisible(visible: boolean): void {
    this.element.hidden = !visible;
    this.#inputWidget.setVisible(visible);
    this.#listWidget.setVisible(visible);
  }

  focus(): void {
    this.#inputWidget.focus();
  }

  #render(): void {
    this.element.dataset.threadId = this.#model.threadId;
    this.#listWidget.render(this.#model.items);
    this.#inputWidget.render({
      phase: this.#model.state,
      error: this.#model.error,
      canInterrupt: this.#model.canInterrupt,
      models: this.#model.models,
      selectedModel: this.#model.selectedModel,
      interaction: this.#model.interaction,
    });
  }
}
