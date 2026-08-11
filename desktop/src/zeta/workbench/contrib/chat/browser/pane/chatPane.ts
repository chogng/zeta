import { DisposableOwner } from "../../../../../base/common/lifecycle.js";
import type { ICommandService } from "../../../../../platform/commands/common/commands.js";
import type { IContextMenuService } from "../../../../../platform/contextview/browser/contextMenu.js";
import type { IContextViewService } from "../../../../../platform/contextview/browser/contextView.js";
import type { IChatService } from "../../../../services/chat/common/chatService.js";
import type { IActiveSessionThread, IUntitledChatSession, IWorkbenchSessionService, SessionId, ThreadId } from "../../../../services/sessions/common/sessionService.js";
import type { ChatInputDelegate } from "../input/chatInput.js";
import { ChatInputPart } from "../input/chatInputPart.js";
import { ChatListWidget } from "../list/chatListWidget.js";
import { ChatPaneModel, type ChatPaneSelection } from "./chatPaneModel.js";

/** Owns the content and interaction state for one local or durable Chat tab. */
export class ChatPane extends DisposableOwner {
  readonly element: HTMLElement;
  private readonly model: ChatPaneModel;
  private readonly listWidget: ChatListWidget;
  private readonly inputPart: ChatInputPart;
  private submittedMessage = false;

  constructor(ownerDocument: Document, panelId: string, chatService: IChatService, selection: ChatPaneSelection, sessionService: IWorkbenchSessionService, contextMenuService: IContextMenuService, contextViewService: IContextViewService, commandService: ICommandService) {
    super();
    this.element = ownerDocument.createElement("div");
    this.element.id = panelId;
    this.element.className = "zeta-chat";
    this.element.setAttribute("role", "tabpanel");
    this.element.hidden = true;
    this.model = this.own(new ChatPaneModel(chatService, selection, sessionService));
    this.listWidget = this.own(new ChatListWidget(ownerDocument));
    const inputDelegate: ChatInputDelegate = {
      send: (text) => this.send(text),
      executeCommand: (invocation) => commandService.executeCommand(invocation.commandId, invocation.argumentsText),
      interrupt: () => this.model.interrupt(),
      selectModel: (model) => this.model.selectModel(model),
      resolveInteraction: (response) => this.model.resolveInteraction(response),
    };
    this.inputPart = this.own(new ChatInputPart(ownerDocument, inputDelegate, contextMenuService, contextViewService));
    this.element.append(this.listWidget.element, this.inputPart.element);
    this.own(this.model.onDidChange(() => this.render()));
    this.defer(() => this.element.remove());
    this.render();
  }

  get sessionId(): SessionId | undefined {
    return this.model.sessionId;
  }

  get untitledSessionId(): string | undefined {
    return this.model.untitledSessionId;
  }

  get threadId(): ThreadId | undefined {
    return this.model.threadId;
  }

  selectThread(active: IActiveSessionThread): Promise<void> {
    if (active.session.sessionId !== this.sessionId) {
      throw new Error(`ChatPane cannot select a Thread from another Session: ${active.session.sessionId}`);
    }
    if (active.threadId !== this.threadId) this.submittedMessage = false;
    return this.model.selectThread(active);
  }

  selectUntitledSession(session: IUntitledChatSession): void {
    if (session.untitledSessionId !== this.untitledSessionId) {
      throw new Error(`ChatPane cannot select another Untitled Chat Session: ${session.untitledSessionId}`);
    }
    this.model.selectUntitledSession(session);
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
    this.inputPart.setVisible(visible);
    this.listWidget.setVisible(visible);
  }

  focus(): void {
    this.inputPart.focus();
  }

  private async send(text: string): Promise<void> {
    this.submittedMessage = true;
    this.updateConversationState();
    try {
      await this.model.send(text);
    } catch (error) {
      if (this.model.items.length === 0) {
        this.submittedMessage = false;
        this.updateConversationState();
      }
      throw error;
    }
  }

  private render(): void {
    this.syncIdentity();
    const items = this.model.items;
    this.updateConversationState(items.length > 0);
    this.listWidget.render(items);
    this.inputPart.render({
      phase: this.model.state,
      error: this.model.error,
      canInterrupt: this.model.canInterrupt,
      models: this.model.models,
      slashCommands: this.model.slashCommands,
      selectedModel: this.model.selectedModel,
      interaction: this.model.interaction,
    });
  }

  private updateConversationState(hasTranscript = this.model.items.length > 0): void {
    const hasConversation = this.submittedMessage || hasTranscript;
    this.element.classList.toggle("empty", !hasConversation);
    this.element.classList.toggle("has-conversation", hasConversation);
  }

  private syncIdentity(): void {
    const sessionId = this.model.sessionId;
    const threadId = this.model.threadId;
    const untitledSessionId = this.model.untitledSessionId;
    if (sessionId) this.element.dataset.sessionId = sessionId;
    else this.element.removeAttribute("data-session-id");
    if (threadId) this.element.dataset.threadId = threadId;
    else this.element.removeAttribute("data-thread-id");
    if (untitledSessionId) this.element.dataset.untitledSessionId = untitledSessionId;
    else this.element.removeAttribute("data-untitled-session-id");
  }
}
