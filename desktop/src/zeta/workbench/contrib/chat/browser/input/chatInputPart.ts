import "./chatInputPart.css";
import { addDisposableListener } from "../../../../../base/browser/dom.js";
import { ButtonActionViewItem, type ActionViewItem } from "../../../../../base/browser/ui/actionbar/actionViewItems.js";
import { DropdownMenuActionViewItem } from "../../../../../base/browser/ui/dropdown/dropdownMenuActionViewItem.js";
import type { IAction } from "../../../../../base/common/actions.js";
import type { Icon } from "../../../../../base/common/icon.js";
import { DisposableOwner, ResettableDisposableGroup } from "../../../../../base/common/lifecycle.js";
import { lxiconsLibrary } from "../../../../../base/common/lxiconsLibrary.js";
import { WorkbenchToolBar } from "../../../../../platform/actions/browser/toolbar.js";
import type { IContextMenuService } from "../../../../../platform/contextview/browser/contextMenu.js";
import type { ModelCatalogEntry } from "../../../../services/chat/common/chatService.js";
import type { ModelRef } from "../../../../services/sessions/common/sessionService.js";
import { DesktopSlashCommands, parseSlashCommandInput, SlashCommandCatalog } from "../../common/slashCommands.js";
import type { ChatInputDelegate, ChatInputState } from "./chatInput.js";
import { ChatInputEditors, type IChatInputEditor } from "./chatInputEditor.js";

type ChatInputMode = "agent" | "plan" | "debug" | "multitask" | "ask";
type ChatInputToolbarPresentation = "mode" | "model" | "attachment" | "send" | "interrupt";

interface ChatInputToolbarState {
  readonly canSubmit: boolean;
  readonly canInterrupt: boolean;
  readonly inputKind: "message" | "command";
  readonly models: readonly ModelCatalogEntry[];
  readonly selectedModel?: ModelRef;
}

const modeOptions: readonly { readonly id: ChatInputMode; readonly label: string }[] = [
  { id: "agent", label: "Agent" },
  { id: "plan", label: "Plan" },
  { id: "debug", label: "Debug" },
  { id: "multitask", label: "Multitask" },
  { id: "ask", label: "Ask" },
];

/** Owns the complete input region and all user-facing interactions for one Chat pane. */
export class ChatInputPart extends DisposableOwner {
  readonly element: HTMLElement;
  private readonly delegate: ChatInputDelegate;
  private readonly interactionListeners = this.own(new ResettableDisposableGroup());
  private readonly status: HTMLDivElement;
  private readonly interaction: HTMLDivElement;
  private readonly inputContainer: HTMLFormElement;
  private readonly input: IChatInputEditor;
  private readonly inputToolbar: WorkbenchToolBar;
  private readonly slashCommands = new SlashCommandCatalog(DesktopSlashCommands, []);
  private state: ChatInputState = { phase: "loading", canInterrupt: false, models: [], slashCommands: [] };
  private toolbarState: ChatInputToolbarState = { canSubmit: false, canInterrupt: false, inputKind: "message", models: [] };
  private serverSlashCommands: ChatInputState["slashCommands"] = [];
  private mode: ChatInputMode = "agent";

  constructor(ownerDocument: Document, delegate: ChatInputDelegate, contextMenuService: IContextMenuService) {
    super();
    this.delegate = delegate;
    this.element = ownerDocument.createElement("div");
    this.element.className = "zeta-chat-input-part";
    this.status = ownerDocument.createElement("div");
    this.status.className = "zeta-chat-status";
    this.status.setAttribute("role", "status");
    this.interaction = ownerDocument.createElement("div");
    this.interaction.className = "zeta-chat-interaction";
    this.interaction.setAttribute("aria-live", "polite");
    this.inputContainer = ownerDocument.createElement("form");
    this.inputContainer.className = "zeta-chat-input-container";
    const editorHost = ownerDocument.createElement("div");
    editorHost.className = "zeta-chat-input-editor-host";
    this.input = this.own(ChatInputEditors.create({
      container: editorHost,
      placeholder: "Ask Zeta",
      ariaLabel: "Chat message",
      slashCommands: this.slashCommands,
    }));
    this.inputToolbar = this.own(new WorkbenchToolBar(contextMenuService, ownerDocument, {
      ariaLabel: "Chat input actions",
      actionViewItemProvider: action => this.createToolbarViewItem(action, contextMenuService),
    }));
    this.inputToolbar.element.classList.add("zeta-chat-input-toolbars");
    this.inputContainer.append(editorHost, this.inputToolbar.element);
    this.element.append(this.status, this.interaction, this.inputContainer);
    this.own(addDisposableListener(this.inputContainer, "focusin", () => this.inputContainer.classList.add("focused")));
    this.own(addDisposableListener(this.inputContainer, "focusout", event => {
      if (this.inputContainer.contains(event.relatedTarget as Node | null)) return;
      this.inputContainer.classList.remove("focused");
    }));
    this.own(addDisposableListener(this.inputContainer, "submit", (event) => {
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
    this.own(this.input.onDidSubmit(() => this.inputContainer.requestSubmit()));
    this.renderToolbarActions();
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
    const state: ChatInputToolbarState = {
      canSubmit: canSubmitIntent && this.state.phase !== "submitting" && !this.state.canInterrupt,
      canInterrupt: this.state.canInterrupt,
      inputKind: input.kind === "message" ? "message" : "command",
      models: this.state.models,
      selectedModel: this.state.selectedModel,
    };
    if (
      state.canSubmit === this.toolbarState.canSubmit &&
      state.canInterrupt === this.toolbarState.canInterrupt &&
      state.inputKind === this.toolbarState.inputKind &&
      state.models === this.toolbarState.models &&
      sameModel(state.selectedModel, this.toolbarState.selectedModel)
    ) return;
    this.toolbarState = state;
    this.renderToolbarActions();
  }

  private renderToolbarActions(): void {
    const mode = modeOptions.find(option => option.id === this.mode) ?? modeOptions[0]!;
    const modeAction = this.toolbarState.inputKind === "command"
      ? new ChatInputAction("zeta.chat.input.command", "Command", "Slash command", lxiconsLibrary.start, false, "mode", () => {})
      : new SelectorAction(
        "zeta.chat.input.mode",
        mode.label,
        `Mode: ${mode.label}`,
        lxiconsLibrary.unlimited,
        "mode",
        () => modeOptions.map(option => new ChatInputAction(
          `zeta.chat.input.mode.${option.id}`,
          option.label,
          `Use ${option.label} mode`,
          undefined,
          true,
          "mode",
          () => {
            this.mode = option.id;
            this.renderToolbarActions();
          },
          option.id === this.mode,
        )),
      );
    const selectedModel = this.toolbarState.models.find(entry => sameModel(entry.model, this.toolbarState.selectedModel));
    const modelAction = new SelectorAction(
      "zeta.chat.input.model",
      selectedModel?.displayName ?? "Model",
      selectedModel ? `Model: ${selectedModel.displayName}` : "Select model",
      undefined,
      "model",
      this.toolbarState.models.map(entry => new ChatInputAction(
        `zeta.chat.input.model.${entry.model.provider}.${entry.model.model}`,
        entry.displayName,
        `Use ${entry.displayName}`,
        undefined,
        true,
        "model",
        () => void this.delegate.selectModel(entry.model),
        sameModel(entry.model, this.toolbarState.selectedModel),
      )),
      this.toolbarState.models.length > 0,
    );
    const attachmentAction = new ChatInputAction(
      "zeta.chat.input.attachment",
      "Attach",
      "Attachments are not available yet",
      lxiconsLibrary.paperclip,
      false,
      "attachment",
      () => {},
    );
    const trailingAction = this.toolbarState.canInterrupt
      ? new ChatInputAction("zeta.chat.input.interrupt", "Stop", "Stop response", lxiconsLibrary.close, true, "interrupt", () => void this.delegate.interrupt())
      : new ChatInputAction(
        "zeta.chat.input.send",
        "Send",
        this.toolbarState.inputKind === "command" ? "Run command" : "Send message",
        lxiconsLibrary.arrowUp,
        this.toolbarState.canSubmit,
        "send",
        () => this.inputContainer.requestSubmit(),
      );
    const inputActions = this.toolbarState.inputKind === "command" ? [modeAction] : [modeAction, modelAction, attachmentAction];
    this.inputToolbar.setActions([...inputActions, trailingAction]);
  }

  private createToolbarViewItem(action: IAction, contextMenuService: IContextMenuService): ActionViewItem | undefined {
    if (!(action instanceof ChatInputAction)) return undefined;
    if (action instanceof SelectorAction) return new ChatInputSelectorViewItem(action, contextMenuService);
    return new ChatInputButtonViewItem(action);
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

class ChatInputAction implements IAction {
  constructor(
    readonly id: string,
    readonly label: string,
    readonly tooltip: string,
    readonly icon: Icon | undefined,
    readonly enabled: boolean,
    readonly presentation: ChatInputToolbarPresentation,
    readonly callback: () => void,
    readonly checked: boolean | undefined = undefined,
  ) {}

  run(): void {
    this.callback();
  }
}

class SelectorAction extends ChatInputAction {
  readonly actions: readonly IAction[] | (() => readonly IAction[]);

  constructor(id: string, label: string, tooltip: string, icon: Icon | undefined, presentation: "mode" | "model", actions: readonly IAction[] | (() => readonly IAction[]), enabled = true) {
    super(id, label, tooltip, icon, enabled, presentation, () => {});
    this.actions = actions;
  }
}

function sameModel(left: ModelRef | undefined, right: ModelRef | undefined): boolean {
  return left === right || (left !== undefined && right !== undefined && left.provider === right.provider && left.model === right.model);
}

class ChatInputSelectorViewItem extends DropdownMenuActionViewItem {
  private readonly presentation: "mode" | "model";

  constructor(action: SelectorAction, contextMenuService: IContextMenuService) {
    super(action, action.actions, contextMenuService);
    this.presentation = action.presentation as "mode" | "model";
  }

  override render(container: HTMLElement): void {
    super.render(container);
    container.classList.add("zeta-chat-input-selector", `zeta-chat-input-${this.presentation}-selector`);
    container.classList.toggle("disabled", !this.action.enabled);
  }
}

class ChatInputButtonViewItem extends ButtonActionViewItem {
  private readonly presentation: ChatInputToolbarPresentation;

  constructor(action: ChatInputAction) {
    super(action);
    this.presentation = action.presentation;
  }

  override render(container: HTMLElement): void {
    super.render(container);
    container.classList.add(`zeta-chat-input-${this.presentation}`);
    container.classList.toggle("disabled", !this.action.enabled);
  }
}
