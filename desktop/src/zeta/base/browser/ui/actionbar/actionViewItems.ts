import { Separator, type IAction } from "../../../common/actions.js";
import type { Icon } from "../../../common/icon.js";
import { DisposableOwner } from "../../../common/lifecycle.js";
import { assertDefined } from "../../../common/types.js";
import { addDisposableListener } from "../../dom.js";
import { setAriaAttribute } from "../aria/aria.js";
import { Button, type ButtonOptions } from "../button/button.js";
import { getHoverDelegate, type IManagedHover } from "../hover/hoverDelegate.js";
import { appendIcon } from "../icon/icon.js";

const ActionHoverGroupId = "actions";

/** Presentation options shared by every ActionBar item representation. */
export interface ActionViewItemOptions {
  /** Allows the host ActionBar to expose this item as a native drag source. */
  readonly draggable?: boolean;
}

/**
 * Browser representation of one action inside an ActionBar.
 *
 * Implementations render into the container owned by their host and own the
 * resources they create for that representation.
 */
export abstract class ActionViewItem extends DisposableOwner {
  protected constructor(
    readonly action: IAction,
    private readonly actionViewItemOptions: ActionViewItemOptions = {},
  ) {
    super();
  }

  get draggable(): boolean {
    return this.actionViewItemOptions.draggable === true;
  }

  abstract render(container: HTMLElement): void;

  /** Controls whether this item is the ActionBar's page-level Tab stop. */
  abstract setTabbable(tabbable: boolean): void;

  focus(): void {}

  /** Creates a Button using the shared delay group for adjacent actions. */
  protected createButton(options: ButtonOptions): Button {
    return this.own(new Button({
      ...options,
      hoverGroupId: options.hoverGroupId ?? ActionHoverGroupId,
    }));
  }

  /** Installs an action tooltip for view items that render a custom target. */
  protected setupHover(target: HTMLElement, content: string): IManagedHover {
    return this.own(getHoverDelegate().setupHover({
      target,
      content,
      groupId: ActionHoverGroupId,
    }));
  }
}

/** Default button representation for a runnable action. */
export class ButtonActionViewItem extends ActionViewItem {
  private _button: Button | undefined;

  constructor(action: IAction, options: ActionViewItemOptions = {}) {
    super(action, options);
  }

  override render(container: HTMLElement): void {
    if (this._button) {
      throw new Error(`Action view item is already rendered: ${this.action.id}`);
    }
    this._button = this.createButton({
      label: this.action.label,
      ownerDocument: container.ownerDocument,
      icon: this.action.icon,
      title: this.action.tooltip,
      enabled: this.action.enabled,
      checked: this.action.checked,
      onClick: () => this.runAction(),
    });
    container.append(this._button.element);
  }

  override focus(): void {
    this.button.element.focus();
  }

  override setTabbable(tabbable: boolean): void {
    this.button.element.tabIndex = tabbable ? 0 : -1;
  }

  protected get button(): Button {
    assertDefined(this._button, `Action view item is not rendered: ${this.action.id}`);
    return this._button;
  }

  protected runAction(): unknown {
    return this.action.run();
  }
}

export interface LabelActionViewItemOptions extends ActionViewItemOptions {
  readonly label?: string;
  readonly icon?: Icon;
  readonly ariaLabel?: string;
  readonly tooltip?: string;
}

/** Compact icon-and-text representation owned by an ActionBar. */
export class LabelActionViewItem extends ActionViewItem {
  private button: HTMLButtonElement | undefined;

  constructor(action: IAction, private readonly options: LabelActionViewItemOptions = {}) {
    super(action, options);
  }

  override render(container: HTMLElement): void {
    if (this.button) {
      throw new Error(`Action view item is already rendered: ${this.action.id}`);
    }
    const button = container.ownerDocument.createElement("button");
    const label = container.ownerDocument.createElement("span");
    button.className = "zeta-action-label";
    button.type = "button";
    button.disabled = !this.action.enabled;
    if (this.action.checked !== undefined) {
      button.classList.toggle("checked", this.action.checked);
      setAriaAttribute(button, "pressed", this.action.checked);
    }
    if (this.options.ariaLabel) {
      setAriaAttribute(button, "label", this.options.ariaLabel);
    }
    const icon = this.options.icon ?? this.action.icon;
    if (icon) {
      const iconContainer = container.ownerDocument.createElement("span");
      iconContainer.className = "zeta-action-label-icon";
      appendIcon(icon, iconContainer);
      button.append(iconContainer);
    }
    label.className = "zeta-action-label-text";
    label.textContent = this.options.label ?? this.action.label;
    button.append(label);
    this.own(addDisposableListener(button, "click", () => this.action.run()));
    this.setupHover(button, this.options.tooltip ?? this.action.tooltip);
    this.defer(() => button.remove());
    this.button = button;
    container.append(button);
  }

  override focus(): void {
    this.button?.focus();
  }

  override setTabbable(tabbable: boolean): void {
    if (this.button) this.button.tabIndex = tabbable ? 0 : -1;
  }
}

/** Non-interactive visual representation of a separator action. */
export class SeparatorActionViewItem extends ActionViewItem {
  private rendered = false;

  constructor(action: Separator) {
    super(action);
  }

  override render(container: HTMLElement): void {
    if (this.rendered) {
      throw new Error(`Action view item is already rendered: ${this.action.id}`);
    }
    this.rendered = true;
    container.classList.add("zeta-action-view-item-separator");
    container.setAttribute("role", "separator");
  }

  override setTabbable(_tabbable: boolean): void {}
}

/** Creates the base representation used when a platform has no override. */
export function createActionViewItem(action: IAction): ActionViewItem {
  return action instanceof Separator
    ? new SeparatorActionViewItem(action)
    : new ButtonActionViewItem(action);
}
