import "./chatInputWidget.css";
import { addDisposableListener } from "../../../../../base/browser/dom.js";
import { DisposableOwner, ResettableDisposableGroup } from "../../../../../base/common/lifecycle.js";
import type { IContextMenuService } from "../../../../../platform/contextview/browser/contextMenu.js";
import type { ChatInputDelegate, ChatInputState } from "./chatInput.js";
import { ChatInputEditors, type IChatInputEditor } from "./chatInputEditor.js";
import { ChatInputToolbar } from "./chatInputToolbar.js";

/** Owns the composer and all user-facing interactions for one Chat pane. */
export class ChatInputWidget extends DisposableOwner {
  readonly element: HTMLElement;
  readonly #delegate: ChatInputDelegate;
  readonly #interactionListeners = this.own(new ResettableDisposableGroup());
  readonly #status: HTMLDivElement;
  readonly #interaction: HTMLDivElement;
  readonly #form: HTMLFormElement;
  readonly #input: IChatInputEditor;
  readonly #toolbar: ChatInputToolbar;
  #state: ChatInputState = { phase: "loading", canInterrupt: false, models: [] };

  constructor(ownerDocument: Document, delegate: ChatInputDelegate, contextMenuService: IContextMenuService) {
    super();
    this.#delegate = delegate;
    this.element = ownerDocument.createElement("div");
    this.element.className = "zeta-chat-input-widget";
    this.#status = ownerDocument.createElement("div");
    this.#status.className = "zeta-chat-status";
    this.#status.setAttribute("role", "status");
    this.#interaction = ownerDocument.createElement("div");
    this.#interaction.className = "zeta-chat-interaction";
    this.#interaction.setAttribute("aria-live", "polite");
    this.#form = ownerDocument.createElement("form");
    this.#form.className = "zeta-chat-composer";
    const inputContainer = ownerDocument.createElement("div");
    inputContainer.className = "zeta-chat-input-editor-host";
    this.#input = this.own(ChatInputEditors.create({
      container: inputContainer,
      placeholder: "Ask Zeta",
      ariaLabel: "Chat message",
    }));
    this.#toolbar = this.own(new ChatInputToolbar(ownerDocument, contextMenuService, {
      submit: () => this.#form.requestSubmit(),
      interrupt: () => void this.#delegate.interrupt(),
      selectModel: (model) => void this.#delegate.selectModel(model),
    }));
    this.#form.append(inputContainer, this.#toolbar.element);
    this.element.append(this.#status, this.#interaction, this.#form);
    this.own(addDisposableListener(this.#form, "submit", (event) => {
      event.preventDefault();
      const value = this.#input.value;
      if (!value.trim()) return;
      this.#input.value = "";
      this.#renderToolbar();
      void this.#delegate.send(value).catch(() => {
        if (!this.#input.value) {
          this.#input.value = value;
          this.#renderToolbar();
        }
      });
    }));
    this.own(this.#input.onDidChange(() => this.#renderToolbar()));
    this.own(this.#input.onDidSubmit(() => this.#form.requestSubmit()));
    this.defer(() => this.element.remove());
  }

  focus(): void {
    this.#input.focus();
  }

  setVisible(visible: boolean): void {
    if (visible) this.#input.layout();
  }

  render(state: ChatInputState): void {
    this.#state = state;
    this.#status.textContent = this.#statusText(state);
    this.#renderInteraction(state);
    this.#renderToolbar();
  }

  #renderToolbar(): void {
    this.#toolbar.render({
      canSubmit: this.#input.value.trim().length > 0 && this.#state.phase !== "submitting" && !this.#state.canInterrupt,
      canInterrupt: this.#state.canInterrupt,
      models: this.#state.models,
      selectedModel: this.#state.selectedModel,
    });
  }

  #renderInteraction(state: ChatInputState): void {
    this.#interactionListeners.clear();
    this.#interaction.replaceChildren();
    const interaction = state.interaction;
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
        this.#interactionListeners.add(addDisposableListener(decline, "click", () => void this.#delegate.resolveInteraction({
          type: "approval",
          response: { decision: "decline" },
        }).catch(() => undefined)));
        this.#interactionListeners.add(addDisposableListener(approve, "click", () => void this.#delegate.resolveInteraction({
          type: "approval",
          response: { decision: "approveOnce" },
        }).catch(() => undefined)));
        break;
      }
      case "userInput": {
        const form = this.element.ownerDocument.createElement("form");
        form.className = "zeta-chat-interaction-form";
        const inputs = new Map<string, HTMLInputElement | HTMLSelectElement>();
        for (const question of request.request.questions) {
          const label = this.element.ownerDocument.createElement("label");
          label.textContent = question.question;
          const input = question.options && !question.allowFreeForm ? this.#questionSelect(question.options) : this.element.ownerDocument.createElement("input");
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
        this.#interactionListeners.add(addDisposableListener(form, "submit", (event) => {
          event.preventDefault();
          const answers: Record<string, { value: string }> = {};
          for (const [id, input] of inputs) answers[id] = { value: input.value };
          void this.#delegate.resolveInteraction({
            type: "userInput",
            response: { answers },
          }).catch(() => undefined);
        }));
        break;
      }
      case "dynamicTool":
        this.#interaction.textContent = `Waiting for dynamic tool: ${request.call.name}`;
        break;
    }
  }

  #interactionButton(label: string, primary = false): HTMLButtonElement {
    const button = this.element.ownerDocument.createElement("button");
    button.type = "button";
    button.textContent = label;
    button.className = primary ? "zeta-chat-send-button" : "zeta-chat-secondary-button";
    return button;
  }

  #questionSelect(options: readonly { readonly label: string }[]): HTMLSelectElement {
    const select = this.element.ownerDocument.createElement("select");
    for (const option of options) {
      const element = this.element.ownerDocument.createElement("option");
      element.value = option.label;
      element.textContent = option.label;
      select.append(element);
    }
    return select;
  }

  #statusText(state: ChatInputState): string {
    if (state.error) return state.error;
    switch (state.phase) {
      case "loading":
        return "Loading chat...";
      case "submitting":
        return "Working...";
      case "error":
        return "Chat is unavailable.";
      case "ready":
        return state.canInterrupt ? "Zeta is working..." : "";
    }
  }
}
