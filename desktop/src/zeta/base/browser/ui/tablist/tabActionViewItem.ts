import { addDisposableListener } from "../../dom.js";
import type { IAction } from "../../../common/actions.js";
import { lxiconsLibrary } from "../../../common/lxiconsLibrary.js";
import { assertDefined } from "../../../common/types.js";
import { ActionBar } from "../actionbar/actionbar.js";
import { ActionViewItem } from "../actionbar/actionViewItems.js";
import { IconLabel } from "../iconlabel/iconlabel.js";
import type { TabListItem } from "./tabList.js";

export const TAB_CLOSE_ACTION_ID = "zeta.tab.close";

/** Internal action representation of one selectable TabList item. */
export class TabAction<T> implements IAction {
  readonly label: string;
  readonly tooltip: string;
  readonly enabled = true;

  constructor(
    readonly tab: TabListItem<T>,
    readonly checked: boolean,
    readonly activate: (value: T) => void,
  ) {
    this.label = tab.label;
    this.tooltip = tab.tooltip ?? tab.label;
  }

  get id(): string {
    return this.tab.id;
  }

  run(): void {
    this.activate(this.tab.value);
  }
}

/** Browser representation responsible for one TabList item's DOM and actions. */
export class TabActionViewItem<T> extends ActionViewItem {
  private readonly tabAction: TabAction<T>;
  private readonly onClose: ((value: T) => void) | undefined;
  private tabElement: HTMLButtonElement | undefined;

  constructor(action: TabAction<T>, onClose: ((value: T) => void) | undefined) {
    super(action);
    this.tabAction = action;
    this.onClose = onClose;
  }

  override render(container: HTMLElement): void {
    if (this.tabElement) {
      throw new Error(`TabList item is already rendered: ${this.action.id}`);
    }
    const item = this.tabAction.tab;
    container.classList.add("zeta-tab");
    container.classList.toggle("checked", this.tabAction.checked);
    container.classList.toggle("icon", item.icon !== undefined);

    const tab = container.ownerDocument.createElement("button");
    this.tabElement = tab;
    tab.id = item.tabId;
    tab.className = "zeta-tab-label";
    tab.type = "button";
    tab.setAttribute("role", "tab");
    tab.setAttribute("aria-selected", String(this.tabAction.checked));
    tab.setAttribute("aria-label", item.ariaLabel ?? item.label);
    if (item.panelId) tab.setAttribute("aria-controls", item.panelId);
    tab.title = this.tabAction.tooltip;
    const label = this.own(new IconLabel({
      label: item.label,
      icon: item.icon,
      ownerDocument: container.ownerDocument,
      title: this.tabAction.tooltip,
    }));
    tab.append(label.element);
    container.append(tab);

    this.own(addDisposableListener(tab, "click", (event) => {
      event.preventDefault();
      event.stopPropagation();
      this.tabAction.run();
    }));
    if (this.onClose) {
      tab.setAttribute("aria-keyshortcuts", "Delete");
      this.own(addDisposableListener(tab, "keydown", (event) => {
        if (event.key !== "Delete") return;
        event.preventDefault();
        event.stopPropagation();
        this.onClose?.(item.value);
      }));
    }
    const actions = [
      ...(item.actions?.items ?? []),
      ...(this.onClose ? [closeTabAction(item, this.onClose)] : []),
    ];
    if (actions.length > 0) {
      const actionBar = this.own(new ActionBar({
        ownerDocument: container.ownerDocument,
        actions,
        ariaLabel: item.actions?.ariaLabel ?? `${item.label} actions`,
      }));
      actionBar.element.classList.add("zeta-tab-actions");
      if (this.onClose) {
        const closeActionContainer = actionBar.element.querySelector<HTMLElement>(`[data-action-id="${TAB_CLOSE_ACTION_ID}"]`);
        if (!closeActionContainer) {
          throw new Error("TabList close action was not rendered");
        }
        closeActionContainer.classList.add("zeta-tab-close-action");
      }
      container.append(actionBar.element);
    }
  }

  override focus(): void {
    this.tab.focus();
  }

  override setTabbable(tabbable: boolean): void {
    this.tab.tabIndex = tabbable ? 0 : -1;
  }

  private get tab(): HTMLButtonElement {
    assertDefined(this.tabElement, `TabList item is not rendered: ${this.action.id}`);
    return this.tabElement;
  }
}

function closeTabAction<T>(item: TabListItem<T>, close: (value: T) => void): IAction {
  const label = `Close ${item.label}`;
  return {
    id: TAB_CLOSE_ACTION_ID,
    label,
    tooltip: label,
    icon: lxiconsLibrary.close,
    enabled: true,
    run: () => close(item.value),
  };
}
