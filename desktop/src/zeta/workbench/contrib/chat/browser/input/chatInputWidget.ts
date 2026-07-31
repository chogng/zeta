import "./chatInputWidget.css";
import { addDisposableListener } from "../../../../../base/browser/dom.js";
import { DisposableOwner, ResettableDisposableGroup } from "../../../../../base/common/lifecycle.js";
import type { IContextMenuService } from "../../../../../platform/contextview/browser/contextMenu.js";
import { DesktopSlashCommands, parseSlashCommandInput, SlashCommandCatalog } from "../../common/slashCommands.js";
import type { ChatInputDelegate, ChatInputState } from "./chatInput.js";
import { ChatInputEditors, type IChatInputEditor } from "./chatInputEditor.js";
import { ChatInputToolbar } from "./chatInputToolbar.js";

/** Owns the composer and all user-facing interactions for one Chat pane. */
export class ChatInputWidget extends DisposableOwner {
  readonly element: HTMLElement;
  private readonly delegate: ChatInputDelegate;
  private readonly interactionListeners = this.own(new ResettableDisposableGroup());
  private readonly status: HTMLDivElement;
  private readonly interaction: HTMLDivElement;
  private readonly form: HTMLFormElement;
  private readonly input: IChatInputEditor;
  private readonly toolbar: ChatInputToolbar;
  private readonly slashCommands = new SlashCommandCatalog(DesktopSlashCommands, []);
  private state: ChatInputState = { phase: "loading", canInterrupt: false, models: [], slashCommands: [] };
  private serverSlashCommands: ChatInputState["slashCommands"] = [];

  constructor(ownerDocument: Document, delegate: ChatInputDelegate, contextMenuService: IContextMenuService) {
    super();
    this.delegate = delegate;
    this.element = ownerDocument.createElement("div");
    this.element.className = "zeta-chat-input-widget";
    this.status = ownerDocument.createElement("div");
    this.status.className = "zeta-chat-status";
    this.status.setAttribute("role", "status");
    this.interaction = ownerDocument.createElement("div");
    this.interaction.className = "zeta-chat-interaction";
    this.interaction.setAttribute("aria-live", "polite");
    this.form = ownerDocument.createElement("form");
    this.form.className = "zeta-chat-composer";
    const inputContainer = ownerDocument.createElement("div");
    inputContainer.className = "zeta-chat-input-editor-host";
    this.input = this.own(ChatInputEditors.create({
      container: inputContainer,
      placeholder: "Ask Zeta",
      ariaLabel: "Chat message",
      slashCommands: this.slashCommands,
    }));
    this.toolbar = this.own(new ChatInputToolbar(ownerDocument, contextMenuService, {
      submit: () => this.form.requestSubmit(),
      interrupt: () => void this.delegate.interrupt(),
      selectModel: (model) => void this.delegate.selectModel(model),
    }));
    this.form.append(inputContainer, this.toolbar.element);
    this.element.append(this.status, this.interaction, this.form);
    this.own(addDisposableListener(this.form, "submit", (event) => {
      event.preventDefault();
      const value = this.input.value;
      if (!value.trim()) return;
      const input = parseSlashCommandInput(value, this.slashCommands);
      if (input.kind === "command" && input.binding.origin === "local") {
        this.submit(value, this.delegate.executeCommand({ commandId: input.binding.actionId, argumentsText: input.argumentsText }));
        return;
      }
      this.submit(value, this.delegate.send(value));
    }));
    this.own(this.input.onDidChange(() => {
      this.status.textContent = this.statusText(this.state);
      this.renderToolbar();
    }));
    this.own(this.input.onDidSubmit(() => this.form.requestSubmit()));
    this.defer(() => this.element.remove());
  }

  private submit(value: string, operation: Promise<void>): void {
    this.input.value = "";
    this.renderToolbar();
    void operation.catch(() => {
      if (!this.input.value) {
        this.input.value = value;
        this.renderToolbar();
      }
    });
  }

  focus(): void {
    this.input.focus();
  }

  setVisible(visible: boolean): void {
    if (visible) this.input.layout();
  }

  render(state: ChatInputState): void {
    if (this.serverSlashCommands !== state.slashCommands) {
      this.slashCommands.setServerCommands(state.slashCommands);
      this.serverSlashCommands = state.slashCommands;
    }
    this.state = state;
    this.status.textContent = this.statusText(state);
    this.renderInteraction(state);
    this.renderToolbar();
  }

  private renderToolbar(): void {
    const input = parseSlashCommandInput(this.input.value, this.slashCommands);
    const canSubmitIntent = input.kind === "message" ? input.text.trim().length > 0 : this.input.value.trim().length > 0;
    this.toolbar.render({
      canSubmit: canSubmitIntent && this.state.phase !== "submitting" && !this.state.canInterrupt,
      canInterrupt: this.state.canInterrupt,
      inputKind: input.kind === "message" ? "message" : "command",
      models: this.state.models,
      selectedModel: this.state.selectedModel,
    });
  }

  private renderInteraction(state: ChatInputState): void {
    this.interactionListeners.clear();
    this.interaction.replaceChildren();
    const interaction = state.interaction;
    if (!interaction) {
      this.interaction.hidden = true;
      return;
    }
    this.interaction.hidden = false;
    const request = interaction.request;
    switch (request.type) {
      case "approval": {
        const reason = this.element.ownerDocument.createElement("p");
        reason.textContent = request.request.reason;
        const actions = this.element.ownerDocument.createElement("div");
        actions.className = "zeta-chat-interaction-actions";
        const decline = this.interactionButton("Decline");
        const approve = this.interactionButton("Approve once", true);
        actions.append(decline, approve);
        this.interaction.append(reason, actions);
        this.interactionListeners.add(addDisposableListener(decline, "click", () => void this.delegate.resolveInteraction({
          type: "approval",
          response: { decision: "decline" },
        }).catch(() => undefined)));
        this.interactionListeners.add(addDisposableListener(approve, "click", () => void this.delegate.resolveInteraction({
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
          const input = question.options && !question.allowFreeForm ? this.questionSelect(question.options) : this.element.ownerDocument.createElement("input");
          input.required = true;
          input.name = question.id;
          label.append(input);
          form.append(label);
          inputs.set(question.id, input);
        }
        const submit = this.interactionButton("Submit", true);
        submit.type = "submit";
        form.append(submit);
        this.interaction.append(form);
        this.interactionListeners.add(addDisposableListener(form, "submit", (event) => {
          event.preventDefault();
          const answers: Record<string, { value: string }> = {};
          for (const [id, input] of inputs) answers[id] = { value: input.value };
          void this.delegate.resolveInteraction({
            type: "userInput",
            response: { answers },
          }).catch(() => undefined);
        }));
        break;
      }
      case "dynamicTool":
        this.interaction.textContent = `Waiting for dynamic tool: ${request.call.name}`;
        break;
    }
  }

  private interactionButton(label: string, primary = false): HTMLButtonElement {
    const button = this.element.ownerDocument.createElement("button");
    button.type = "button";
    button.textContent = label;
    button.className = primary ? "zeta-chat-send-button" : "zeta-chat-secondary-button";
    return button;
  }

  private questionSelect(options: readonly { readonly label: string }[]): HTMLSelectElement {
    const select = this.element.ownerDocument.createElement("select");
    for (const option of options) {
      const element = this.element.ownerDocument.createElement("option");
      element.value = option.label;
      element.textContent = option.label;
      select.append(element);
    }
    return select;
  }

  private statusText(state: ChatInputState): string {
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
