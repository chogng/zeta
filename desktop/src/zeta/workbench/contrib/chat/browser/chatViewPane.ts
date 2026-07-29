import "./media/chat.css";
import {
  addDisposableListener,
} from "../../../../base/browser/dom.js";
import { MarkdownElement } from "../../../../base/browser/markdownRenderer.js";
import {
  ResettableDisposableGroup,
} from "../../../../base/common/lifecycle.js";
import type {
  ZetaRendererApi,
} from "../../../../platform/app-server/common/renderer-api.js";
import {
  ViewPane,
  type IViewPaneOptions,
} from "../../../browser/parts/views/viewPane.js";
import type {
  IWorkbenchSessionService,
} from "../../../services/sessions/common/sessionService.js";
import {
  ChatViewModel,
} from "./chatViewModel.js";
import type {
  IChatDisplayItem,
} from "./chatDisplayItems.js";

/** Transcript and composer for the active Workbench Thread. */
export class ChatViewPane extends ViewPane {
  readonly #model: ChatViewModel;
  readonly #renderedItems =
    this.own(new ResettableDisposableGroup());
  readonly #interactionListeners =
    this.own(new ResettableDisposableGroup());
  readonly #transcript: HTMLDivElement;
  readonly #status: HTMLDivElement;
  readonly #interaction: HTMLDivElement;
  readonly #form: HTMLFormElement;
  readonly #input: HTMLTextAreaElement;
  readonly #sendButton: HTMLButtonElement;
  readonly #interruptButton: HTMLButtonElement;

  constructor(
    options: IViewPaneOptions,
    api: ZetaRendererApi,
    sessionService: IWorkbenchSessionService,
  ) {
    super(options);
    this.element.classList.add("zeta-chat-view-pane");
    this.titleElement.hidden = true;
    this.contentElement.classList.add("zeta-chat");
    this.#model = this.own(new ChatViewModel(api, sessionService));

    const toolbar = options.ownerDocument.createElement("div");
    toolbar.className = "zeta-chat-toolbar";
    const newChat = options.ownerDocument.createElement("button");
    newChat.type = "button";
    newChat.className = "zeta-chat-secondary-button";
    newChat.textContent = "New Chat";
    toolbar.append(newChat);

    this.#transcript = options.ownerDocument.createElement("div");
    this.#transcript.className = "zeta-chat-transcript";
    this.#transcript.setAttribute("role", "log");
    this.#transcript.setAttribute("aria-label", "Chat transcript");
    this.#transcript.setAttribute("aria-live", "polite");

    this.#status = options.ownerDocument.createElement("div");
    this.#status.className = "zeta-chat-status";
    this.#status.setAttribute("role", "status");

    this.#interaction = options.ownerDocument.createElement("div");
    this.#interaction.className = "zeta-chat-interaction";
    this.#interaction.setAttribute("aria-live", "polite");

    this.#form = options.ownerDocument.createElement("form");
    this.#form.className = "zeta-chat-composer";
    this.#input = options.ownerDocument.createElement("textarea");
    this.#input.rows = 3;
    this.#input.placeholder = "Ask Zeta";
    this.#input.setAttribute("aria-label", "Chat message");
    const actions = options.ownerDocument.createElement("div");
    actions.className = "zeta-chat-composer-actions";
    this.#interruptButton = options.ownerDocument.createElement("button");
    this.#interruptButton.type = "button";
    this.#interruptButton.className = "zeta-chat-secondary-button";
    this.#interruptButton.textContent = "Stop";
    this.#sendButton = options.ownerDocument.createElement("button");
    this.#sendButton.type = "submit";
    this.#sendButton.className = "zeta-chat-send-button";
    this.#sendButton.textContent = "Send";
    actions.append(this.#interruptButton, this.#sendButton);
    this.#form.append(this.#input, actions);
    this.contentElement.append(
      toolbar,
      this.#transcript,
      this.#status,
      this.#interaction,
      this.#form,
    );

    this.own(this.#model.onDidChange(() => this.#render()));
    this.own(addDisposableListener(newChat, "click", () => {
      void this.#model.startNewChat();
    }));
    this.own(addDisposableListener(
      this.#interruptButton,
      "click",
      () => void this.#model.interrupt(),
    ));
    this.own(addDisposableListener(
      this.#form,
      "submit",
      (event) => {
        event.preventDefault();
        const value = this.#input.value;
        if (!value.trim()) return;
        this.#input.value = "";
        void this.#model.send(value).catch(() => {
          this.#input.value = value;
        });
      },
    ));
    this.own(addDisposableListener(
      this.#input,
      "keydown",
      (event: KeyboardEvent) => {
        if (
          event.key !== "Enter" ||
          event.shiftKey ||
          event.isComposing
        ) return;
        event.preventDefault();
        this.#form.requestSubmit();
      },
    ));
    this.#render();
  }

  override focus(): void {
    this.#input.focus();
  }

  #render(): void {
    const shouldFollow =
      this.#transcript.scrollHeight -
        this.#transcript.scrollTop -
        this.#transcript.clientHeight < 48;
    this.#renderedItems.clear();
    this.#transcript.replaceChildren(
      ...this.#model.items.map((item) => this.#renderItem(item)),
    );
    if (this.#model.items.length === 0) {
      const empty = this.element.ownerDocument.createElement("div");
      empty.className = "zeta-chat-empty";
      empty.textContent = this.#model.thread
        ? "Start a conversation."
        : "Start a new chat to begin.";
      this.#transcript.append(empty);
    }
    if (shouldFollow) {
      this.#transcript.scrollTop = this.#transcript.scrollHeight;
    }
    this.#status.textContent = this.#statusText();
    this.#renderInteraction();
    const submitting = this.#model.state === "submitting";
    this.#sendButton.disabled = submitting || this.#model.canInterrupt;
    this.#interruptButton.hidden = !this.#model.canInterrupt;
  }

  #renderInteraction(): void {
    this.#interactionListeners.clear();
    this.#interaction.replaceChildren();
    const interaction = this.#model.interaction;
    if (!interaction) {
      this.#interaction.hidden = true;
      return;
    }
    this.#interaction.hidden = false;
    const request = interaction.request;
    switch (request.type) {
      case "approval": {
        const reason = this.element.ownerDocument.createElement("p");
        reason.textContent = request.request.reason;
        const actions = this.element.ownerDocument.createElement("div");
        actions.className = "zeta-chat-interaction-actions";
        const decline = this.#interactionButton("Decline");
        const approve = this.#interactionButton("Approve once", true);
        actions.append(decline, approve);
        this.#interaction.append(reason, actions);
        this.#interactionListeners.add(addDisposableListener(
          decline,
          "click",
          () => void this.#model.resolveInteraction({
            type: "approval",
            response: { decision: "decline" },
          }).catch(() => undefined),
        ));
        this.#interactionListeners.add(addDisposableListener(
          approve,
          "click",
          () => void this.#model.resolveInteraction({
            type: "approval",
            response: { decision: "approveOnce" },
          }).catch(() => undefined),
        ));
        break;
      }
      case "userInput": {
        const form = this.element.ownerDocument.createElement("form");
        form.className = "zeta-chat-interaction-form";
        const inputs = new Map<
          string,
          HTMLInputElement | HTMLSelectElement
        >();
        for (const question of request.request.questions) {
          const label = this.element.ownerDocument.createElement("label");
          label.textContent = question.question;
          const input = question.options && !question.allowFreeForm
            ? this.#questionSelect(question.options)
            : this.element.ownerDocument.createElement("input");
          input.required = true;
          input.name = question.id;
          label.append(input);
          form.append(label);
          inputs.set(question.id, input);
        }
        const submit = this.#interactionButton("Submit", true);
        submit.type = "submit";
        form.append(submit);
        this.#interaction.append(form);
        this.#interactionListeners.add(addDisposableListener(
          form,
          "submit",
          (event) => {
            event.preventDefault();
            const answers: Record<string, { value: string }> = {};
            for (const [id, input] of inputs) {
              answers[id] = { value: input.value };
            }
            void this.#model.resolveInteraction({
              type: "userInput",
              response: { answers },
            }).catch(() => undefined);
          },
        ));
        break;
      }
      case "dynamicTool":
        this.#interaction.textContent =
          `Waiting for dynamic tool: ${request.call.name}`;
        break;
    }
  }

  #interactionButton(
    label: string,
    primary = false,
  ): HTMLButtonElement {
    const button = this.element.ownerDocument.createElement("button");
    button.type = "button";
    button.textContent = label;
    button.className = primary
      ? "zeta-chat-send-button"
      : "zeta-chat-secondary-button";
    return button;
  }

  #questionSelect(
    options: readonly { readonly label: string }[],
  ): HTMLSelectElement {
    const select = this.element.ownerDocument.createElement("select");
    for (const option of options) {
      const element = this.element.ownerDocument.createElement("option");
      element.value = option.label;
      element.textContent = option.label;
      select.append(element);
    }
    return select;
  }

  #renderItem(item: IChatDisplayItem): HTMLElement {
    const article = this.element.ownerDocument.createElement("article");
    article.className = `zeta-chat-item zeta-chat-item-${item.type}`;
    article.dataset.itemId = item.id;
    if (item.transient) article.dataset.transient = "true";
    if (item.isError) article.classList.add("error");
    const label = this.element.ownerDocument.createElement("div");
    label.className = "zeta-chat-item-label";
    label.textContent = itemLabel(item);
    article.append(label);
    if (
      item.type === "agentMessage" ||
      item.type === "reasoning" ||
      item.type === "plan"
    ) {
      const markdown = this.#renderedItems.add(new MarkdownElement({
        ownerDocument: this.element.ownerDocument,
        markdown: item.text,
        breaks: true,
      }));
      article.append(markdown.element);
    } else {
      const content = this.element.ownerDocument.createElement("pre");
      content.textContent = item.text;
      article.append(content);
    }
    return article;
  }

  #statusText(): string {
    if (this.#model.error) return this.#model.error;
    switch (this.#model.state) {
      case "loading":
        return "Loading chat…";
      case "submitting":
        return "Working…";
      case "error":
        return "Chat is unavailable.";
      case "ready":
        return this.#model.canInterrupt ? "Zeta is working…" : "";
    }
  }
}

function itemLabel(item: IChatDisplayItem): string {
  switch (item.type) {
    case "userMessage":
    case "userImage":
      return "You";
    case "agentMessage":
      return "Zeta";
    case "reasoning":
      return "Reasoning";
    case "plan":
      return "Plan";
    case "toolCall":
      return "Tool call";
    case "toolResult":
      return item.isError ? "Tool error" : "Tool result";
  }
}
