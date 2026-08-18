import { addDisposableListener } from "../../../../base/browser/dom.js";
import type { IContextMenuProvider } from "../../../../base/browser/contextmenu.js";
import { ActionViewItem } from "../../../../base/browser/ui/actionbar/actionViewItems.js";
import { IconLabel } from "../../../../base/browser/ui/iconlabel/iconlabel.js";
import type { IAction } from "../../../../base/common/actions.js";
import type { Icon } from "../../../../base/common/icon.js";
import { assertDefined } from "../../../../base/common/types.js";

/** Inputs for one View Container selector rendered by a CompositeBar. */
export interface CompositeBarActionOptions {
  readonly id: string;
  readonly label: string;
  readonly tooltip?: string;
  readonly icon?: Icon;
  readonly tabId: string;
  readonly panelId?: string;
  readonly checked: boolean;
  readonly onActivate: (compositeId: string) => void;
}

/** Action model for a View Container selector; CompositeBar owns its lifecycle. */
export class CompositeBarAction implements IAction {
  readonly enabled = true;

  constructor(readonly options: CompositeBarActionOptions) {}

  get id(): string {
    return this.options.id;
  }

  get label(): string {
    return this.options.label;
  }

  get tooltip(): string {
    return this.options.tooltip ?? this.options.label;
  }

  get icon(): Icon | undefined {
    return this.options.icon;
  }

  get checked(): boolean {
    return this.options.checked;
  }

  run(): void {
    this.options.onActivate(this.options.id);
  }
}

/** DOM representation of one CompositeBar action inside its ActionBar tablist. */
export class CompositeBarActionViewItem extends ActionViewItem {
  private containerElement: HTMLElement | undefined;

  constructor(private readonly compositeAction: CompositeBarAction) {
    super(compositeAction, { draggable: true });
  }

  override render(container: HTMLElement): void {
    if (this.containerElement) {
      throw new Error(`CompositeBar action is already rendered: ${this.action.id}`);
    }
    const options = this.compositeAction.options;
    this.containerElement = container;
    container.classList.add("zeta-composite-bar-item");
    container.classList.add("zeta-composite-bar-destination");
    container.classList.toggle("checked", options.checked);
    container.id = options.tabId;
    container.setAttribute("role", "tab");
    container.setAttribute("aria-selected", String(options.checked));
    container.setAttribute("aria-label", options.label);
    if (options.panelId) container.setAttribute("aria-controls", options.panelId);
    this.setupHover(container, this.compositeAction.tooltip);
    const action = container.ownerDocument.createElement("span");
    action.className = "zeta-composite-bar-action";
    const label = this.own(new IconLabel({
      label: options.label,
      icon: options.icon,
      ownerDocument: container.ownerDocument,
    }));
    action.append(label.element);
    container.append(action);
    this.own(addDisposableListener(container, "click", (event) => {
      event.preventDefault();
      event.stopPropagation();
      this.compositeAction.run();
    }));
    this.own(addDisposableListener(container, "keydown", (event) => {
      if (event.key !== "Enter" && event.key !== " ") return;
      event.preventDefault();
      event.stopPropagation();
      this.compositeAction.run();
    }));
  }

  override focus(): void {
    this.container.focus();
  }

  override setTabbable(tabbable: boolean): void {
    this.container.tabIndex = tabbable ? 0 : -1;
  }

  private get container(): HTMLElement {
    assertDefined(this.containerElement, `CompositeBar action is not rendered: ${this.action.id}`);
    return this.containerElement;
  }
}

/** Overflow selector that remains a Composite tab while opening a menu of hidden destinations. */
export class CompositeBarOverflowViewItem extends ActionViewItem {
  private containerElement: HTMLElement | undefined;

  constructor(
    action: IAction,
    private readonly getActions: () => readonly IAction[],
    private readonly contextMenuProvider: IContextMenuProvider,
  ) {
    super(action);
  }

  override render(container: HTMLElement): void {
    if (this.containerElement) {
      throw new Error(`CompositeBar overflow action is already rendered: ${this.action.id}`);
    }
    this.containerElement = container;
    container.classList.add("zeta-composite-bar-item");
    container.classList.add("zeta-composite-bar-overflow");
    container.setAttribute("role", "tab");
    container.setAttribute("aria-selected", "false");
    container.setAttribute("aria-label", this.action.label);
    container.setAttribute("aria-haspopup", "menu");
    container.setAttribute("aria-expanded", "false");
    this.setupHover(container, this.action.tooltip);
    const action = container.ownerDocument.createElement("span");
    action.className = "zeta-composite-bar-action";
    const label = this.own(new IconLabel({
      label: this.action.label,
      icon: this.action.icon,
      ownerDocument: container.ownerDocument,
    }));
    action.append(label.element);
    container.append(action);
    this.own(addDisposableListener(container, "click", (event) => {
      event.preventDefault();
      event.stopPropagation();
      this.show();
    }));
    this.own(addDisposableListener(container, "keydown", (event) => {
      if (event.key !== "Enter" && event.key !== " " && event.key !== "ArrowDown" && event.key !== "ArrowUp") return;
      event.preventDefault();
      event.stopPropagation();
      this.show();
    }));
  }

  override focus(): void {
    this.container.focus();
  }

  override setTabbable(tabbable: boolean): void {
    this.container.tabIndex = tabbable ? 0 : -1;
  }

  private show(): void {
    const actions = this.getActions();
    if (actions.length === 0 || this.container.getAttribute("aria-expanded") === "true") return;
    this.container.setAttribute("aria-expanded", "true");
    try {
      this.contextMenuProvider.showContextMenu({
        anchor: this.container,
        actions,
        onHide: () => this.container.setAttribute("aria-expanded", "false"),
      });
    } catch (error) {
      this.container.setAttribute("aria-expanded", "false");
      throw error;
    }
  }

  private get container(): HTMLElement {
    assertDefined(this.containerElement, `CompositeBar overflow action is not rendered: ${this.action.id}`);
    return this.containerElement;
  }
}
