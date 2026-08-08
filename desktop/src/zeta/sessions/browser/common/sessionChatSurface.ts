import "../../../workbench/contrib/chat/browser/media/chat.css";
import { addDisposableListener } from "../../../base/browser/dom.js";
import { DisposableOwner, ResettableDisposableGroup } from "../../../base/common/lifecycle.js";
import { ChatListWidget } from "../../../workbench/contrib/chat/browser/list/chatListWidget.js";
import { ChatPaneModel, type ChatPaneSelection } from "../../../workbench/contrib/chat/browser/pane/chatPaneModel.js";
import type { IChatService } from "../../../workbench/services/chat/common/chatService.js";
import type { IWorkbenchSessionService } from "../../../workbench/services/sessions/common/sessionService.js";

/** Real App Server chat surface embedded by a dedicated Sessions workbench. */
export class SessionChatSurface extends DisposableOwner {
  readonly element: HTMLElement;
  private readonly sessionService: IWorkbenchSessionService;
  private readonly chatService: IChatService;
  private readonly list: ChatListWidget;
  private readonly form: HTMLFormElement;
  private readonly input: HTMLTextAreaElement;
  private readonly sendButton: HTMLButtonElement;
  private readonly status: HTMLParagraphElement;
  private readonly modelResources = this.own(new ResettableDisposableGroup());
  private model: ChatPaneModel | undefined;
  private selectionKey: string | undefined;

  constructor(ownerDocument: Document, sessionService: IWorkbenchSessionService, chatService: IChatService, placeholder: string, defaultSessionTitle: string) {
    super();
    this.sessionService = sessionService;
    this.chatService = chatService;
    this.element = ownerDocument.createElement("section");
    this.element.className = "zeta-sessions-chat-surface";
    this.list = this.own(new ChatListWidget(ownerDocument));
    this.list.setVisible(true);
    this.status = ownerDocument.createElement("p");
    this.status.className = "zeta-sessions-chat-status";
    this.form = ownerDocument.createElement("form");
    this.form.className = "zeta-sessions-chat-form";
    this.input = ownerDocument.createElement("textarea");
    this.input.className = "zeta-sessions-chat-input";
    this.input.placeholder = placeholder;
    this.input.rows = 3;
    this.input.setAttribute("aria-label", placeholder);
    this.sendButton = ownerDocument.createElement("button");
    this.sendButton.type = "submit";
    this.sendButton.className = "zeta-sessions-button zeta-sessions-primary-button";
    this.sendButton.textContent = "Send";
    this.form.append(this.input, this.sendButton);
    this.element.append(this.list.element, this.status, this.form);
    this.own(addDisposableListener(this.form, "submit", (event) => {
      event.preventDefault();
      void this.send(this.input.value);
    }));
    this.own(addDisposableListener(this.input, "keydown", (event) => {
      if (event.key !== "Enter" || event.shiftKey) return;
      event.preventDefault();
      void this.send(this.input.value);
    }));
    this.own(sessionService.onDidChange(() => this.sync(defaultSessionTitle)));
    this.sync(defaultSessionTitle);
  }

  async sendPrompt(prompt: string): Promise<void> {
    this.input.value = prompt;
    await this.send(prompt);
  }

  focus(): void {
    this.input.focus({ preventScroll: true });
  }

  private sync(defaultSessionTitle: string): void {
    const selection = this.currentSelection(defaultSessionTitle);
    const key = selection.kind === "session"
      ? `session:${selection.active.session.sessionId}:${selection.active.threadId}`
      : `untitled:${selection.session.untitledSessionId}`;
    if (key === this.selectionKey && this.model) return;
    this.modelResources.clear();
    this.selectionKey = key;
    this.model = this.modelResources.add(new ChatPaneModel(this.chatService, selection, this.sessionService));
    this.modelResources.add(this.model.onDidChange(() => this.render()));
    this.render();
  }

  private currentSelection(defaultSessionTitle: string): ChatPaneSelection {
    const active = this.sessionService.active;
    if (active) return { kind: "session", active };
    const untitled = this.sessionService.activeUntitledSession ?? this.sessionService.createUntitledSession(defaultSessionTitle);
    return { kind: "untitled", session: untitled };
  }

  private async send(value: string): Promise<void> {
    const input = value.trim();
    if (!input || !this.model) return;
    this.sendButton.disabled = true;
    try {
      await this.model.send(input);
      this.input.value = "";
    } catch {
      // ChatPaneModel has already projected the operation error into the surface.
    } finally {
      this.render();
    }
  }

  private render(): void {
    const model = this.model;
    if (!model) return;
    this.list.render(model.items);
    this.sendButton.disabled = model.state === "loading" || model.state === "submitting" || model.canInterrupt;
    this.status.textContent = chatStatus(model.state, model.error, model.canInterrupt);
  }
}

function chatStatus(state: "loading" | "ready" | "submitting" | "error", error: string | undefined, canInterrupt: boolean): string {
  if (error) return error;
  if (canInterrupt || state === "submitting") return "Agent is working…";
  if (state === "loading") return "Loading session…";
  return "";
}
