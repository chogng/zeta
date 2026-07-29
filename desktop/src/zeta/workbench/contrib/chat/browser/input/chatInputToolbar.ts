import { ButtonActionViewItem, type ActionViewItem } from "../../../../../base/browser/ui/actionbar/actionViewItems.js";
import { DropdownMenuActionViewItem } from "../../../../../base/browser/ui/dropdown/dropdownMenuActionViewItem.js";
import type { IAction } from "../../../../../base/common/actions.js";
import type { ModelCatalogEntry, ModelRef } from "../../../../../../../generated/app-server/types.js";
import type { Icon } from "../../../../../base/common/icon.js";
import { DisposableOwner } from "../../../../../base/common/lifecycle.js";
import { LxIcon } from "../../../../../base/common/lxicons.js";
import { WorkbenchToolBar } from "../../../../../platform/actions/browser/toolbar.js";
import type { IContextMenuService } from "../../../../../platform/contextview/browser/contextMenu.js";

export type ChatInputMode = "agent" | "plan" | "debug" | "multitask" | "ask";

export interface ChatInputToolbarDelegate {
  submit(): void;
  interrupt(): void;
  selectModel(model: ModelRef): void;
}

export interface ChatInputToolbarState {
  readonly canSubmit: boolean;
  readonly canInterrupt: boolean;
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

type ChatInputToolbarPresentation = "mode" | "model" | "attachment" | "send" | "interrupt";

/** Owns the fixed actions and selector state shown beneath the Chat textarea. */
export class ChatInputToolbar extends DisposableOwner {
  readonly element: HTMLElement;
  readonly #toolbar: WorkbenchToolBar;
  readonly #delegate: ChatInputToolbarDelegate;
  #mode: ChatInputMode = "agent";
  #state: ChatInputToolbarState = { canSubmit: false, canInterrupt: false, models: [] };

  constructor(ownerDocument: Document, contextMenuService: IContextMenuService, delegate: ChatInputToolbarDelegate) {
    super();
    this.#delegate = delegate;
    this.#toolbar = this.own(new WorkbenchToolBar(contextMenuService, ownerDocument, {
      ariaLabel: "Chat input actions",
      actionViewItemProvider: (action) => this.#createViewItem(action, contextMenuService),
    }));
    this.element = this.#toolbar.element;
    this.element.classList.add("zeta-chat-input-toolbar");
    this.#render();
  }

  render(state: ChatInputToolbarState): void {
    if (
      state.canSubmit === this.#state.canSubmit &&
      state.canInterrupt === this.#state.canInterrupt &&
      state.models === this.#state.models &&
      sameModel(state.selectedModel, this.#state.selectedModel)
    ) return;
    this.#state = state;
    this.#render();
  }

  #render(): void {
    const mode = modeOptions.find((option) => option.id === this.#mode) ?? modeOptions[0]!;
    const modeAction = new SelectorAction(
      "zeta.chat.input.mode",
      mode.label,
      `Mode: ${mode.label}`,
      LxIcon.agent,
      "mode",
      () => modeOptions.map((option) => new ChatInputAction(
        `zeta.chat.input.mode.${option.id}`,
        option.label,
        `Use ${option.label} mode`,
        undefined,
        true,
        "mode",
        () => {
          this.#mode = option.id;
          this.#render();
        },
        option.id === this.#mode,
      )),
    );
    const selectedModel = this.#state.models.find((entry) => sameModel(entry.model, this.#state.selectedModel));
    const modelAction = new SelectorAction(
      "zeta.chat.input.model",
      selectedModel?.displayName ?? "Model",
      selectedModel ? `Model: ${selectedModel.displayName}` : "Select model",
      LxIcon.model,
      "model",
      this.#state.models.map((entry) => new ChatInputAction(
        `zeta.chat.input.model.${entry.model.provider}.${entry.model.model}`,
        entry.displayName,
        `Use ${entry.displayName}`,
        undefined,
        true,
        "model",
        () => this.#delegate.selectModel(entry.model),
        sameModel(entry.model, this.#state.selectedModel),
      )),
      this.#state.models.length > 0,
    );
    const attachmentAction = new ChatInputAction(
      "zeta.chat.input.attachment",
      "Attach",
      "Attachments are not available yet",
      LxIcon.add,
      false,
      "attachment",
      () => {},
    );
    const trailingAction = this.#state.canInterrupt
      ? new ChatInputAction(
        "zeta.chat.input.interrupt",
        "Stop",
        "Stop response",
        LxIcon.close,
        true,
        "interrupt",
        () => this.#delegate.interrupt(),
      )
      : new ChatInputAction(
        "zeta.chat.input.send",
        "Send",
        "Send message",
        LxIcon.arrowUp,
        this.#state.canSubmit,
        "send",
        () => this.#delegate.submit(),
      );
    this.#toolbar.setActions([modeAction, modelAction, attachmentAction, trailingAction]);
  }

  #createViewItem(action: IAction, contextMenuService: IContextMenuService): ActionViewItem | undefined {
    if (!(action instanceof ChatInputAction)) return undefined;
    if (action instanceof SelectorAction) {
      return new ChatInputSelectorViewItem(action, contextMenuService);
    }
    return new ChatInputButtonViewItem(action);
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

  constructor(id: string, label: string, tooltip: string, icon: Icon, presentation: "mode" | "model", actions: readonly IAction[] | (() => readonly IAction[]), enabled = true) {
    super(id, label, tooltip, icon, enabled, presentation, () => {});
    this.actions = actions;
  }
}

function sameModel(left: ModelRef | undefined, right: ModelRef | undefined): boolean {
  return left === right || (left !== undefined && right !== undefined && left.provider === right.provider && left.model === right.model);
}

class ChatInputSelectorViewItem extends DropdownMenuActionViewItem {
  readonly #presentation: "mode" | "model";

  constructor(action: SelectorAction, contextMenuService: IContextMenuService) {
    super(action, action.actions, contextMenuService);
    this.#presentation = action.presentation as "mode" | "model";
  }

  override render(container: HTMLElement): void {
    super.render(container);
    container.classList.add("zeta-chat-input-selector", `zeta-chat-input-${this.#presentation}-selector`);
  }
}

class ChatInputButtonViewItem extends ButtonActionViewItem {
  readonly #presentation: ChatInputToolbarPresentation;

  constructor(action: ChatInputAction) {
    super(action);
    this.#presentation = action.presentation;
  }

  override render(container: HTMLElement): void {
    super.render(container);
    container.classList.add(`zeta-chat-input-${this.#presentation}`);
  }
}
