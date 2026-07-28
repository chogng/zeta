import {
  type IAction,
  Separator,
  SubmenuAction,
} from "../../../base/common/actions.js";
import {
  ActionViewItem,
  ButtonActionViewItem,
  SeparatorActionViewItem,
} from "../../../base/browser/ui/actionbar/actionViewItems.js";
import {
  KeybindingLabel,
} from "../../../base/browser/ui/keybindinglabel/keybindinglabel.js";
import {
  AnchorAxisAlignment,
  AnchorPosition,
  ContextView,
  ContextViewFocusRestore,
} from "../../../base/browser/ui/contextview/contextview.js";
import {
  FocusNavigationBoundary,
  FocusNavigationDirection,
  focusFirst,
  focusLast,
  moveFocus,
} from "../../../base/browser/focus.js";
import { addDisposableListener } from "../../../base/browser/dom.js";
import { DisposableOwner } from "../../../base/common/lifecycle.js";
import type {
  ResolvedKeybinding,
} from "../../../base/common/keybindings.js";

export interface MenuEntryActionViewItemOptions {
  readonly onDidSelect?: () => void;
  readonly submenuLayer?: number;
  readonly keybinding?: ResolvedKeybinding;
  readonly getKeybinding?: (
    action: IAction,
  ) => ResolvedKeybinding | undefined;
}

/** Button view item for an action resolved from a menu contribution. */
export class MenuEntryActionViewItem extends ButtonActionViewItem {
  readonly #onDidSelect: (() => void) | undefined;
  readonly #keybinding: ResolvedKeybinding | undefined;

  constructor(
    action: IAction,
    options: MenuEntryActionViewItemOptions = {},
  ) {
    super(action);
    this.#onDidSelect = options.onDidSelect;
    this.#keybinding = options.keybinding;
  }

  override render(container: HTMLElement): void {
    super.render(container);
    if (!this.#keybinding) return;
    const label = this.own(new KeybindingLabel({
      keybinding: this.#keybinding,
      ownerDocument: container.ownerDocument,
    }));
    label.element.classList.add("zeta-menu-keybinding");
    this.button.element.append(label.element);
  }

  protected override runAction(): unknown {
    try {
      return super.runAction();
    } finally {
      this.#onDidSelect?.();
    }
  }
}

/** Button view item that opens a nested menu for a submenu action. */
export class SubmenuActionViewItem extends ButtonActionViewItem {
  readonly #submenuAction: SubmenuAction;
  readonly #onDidSelect: (() => void) | undefined;
  readonly #submenuLayer: number;
  readonly #getKeybinding:
    | ((action: IAction) => ResolvedKeybinding | undefined)
    | undefined;
  #contextView: ContextView | undefined;
  #menu: ActionMenu | undefined;
  #open = false;

  constructor(
    action: SubmenuAction,
    options: MenuEntryActionViewItemOptions = {},
  ) {
    super(action);
    this.#submenuAction = action;
    this.#onDidSelect = options.onDidSelect;
    this.#submenuLayer = options.submenuLayer ?? 20;
    this.#getKeybinding = options.getKeybinding;
  }

  override render(container: HTMLElement): void {
    super.render(container);
    const ownerDocument = container.ownerDocument;
    this.#contextView = this.own(new ContextView(ownerDocument));
    this.#menu = this.own(new ActionMenu({
      actions: this.#submenuAction.actions,
      ownerDocument,
      layer: this.#submenuLayer,
      getKeybinding: this.#getKeybinding,
      onDidSelect: () => {
        this.#hide();
        this.#onDidSelect?.();
      },
      onDidRequestClose: () => {
        this.#hide();
        this.button.element.focus();
      },
    }));
    this.own(this.#contextView.onDidHide(() => {
      this.#open = false;
      this.button.element.setAttribute("aria-expanded", "false");
    }));
    this.button.element.setAttribute("aria-haspopup", "menu");
    this.button.element.setAttribute("aria-expanded", "false");
    const indicator = ownerDocument.createElement("span");
    indicator.className = "zeta-submenu-indicator";
    indicator.textContent = "›";
    indicator.setAttribute("aria-hidden", "true");
    this.button.element.append(indicator);
  }

  protected override runAction(): void {
    if (this.#open) {
      this.#hide();
      return;
    }
    if (!this.#contextView || !this.#menu) return;
    this.#contextView.show({
      anchor: this.button.element,
      content: this.#menu.element,
      anchorAxisAlignment: AnchorAxisAlignment.Horizontal,
      anchorPosition: AnchorPosition.Below,
      gap: 2,
      layer: this.#submenuLayer,
      focusRestore: ContextViewFocusRestore.Previous,
      isTargetWithin: (target) => this.#menu?.contains(target) ?? false,
    });
    this.#open = true;
    this.button.element.setAttribute("aria-expanded", "true");
    this.#menu.focusFirst();
  }

  #hide(): void {
    if (!this.#open) return;
    this.#open = false;
    this.#contextView?.hide();
  }

  contains(target: Node): boolean {
    return this.#contextView?.element.contains(target) === true ||
      this.#menu?.contains(target) === true;
  }

  ownsTrigger(target: Element | null): boolean {
    return target === this.button.element;
  }

  openFromKeyboard(): void {
    if (!this.#open) this.runAction();
  }
}

/** Creates the platform representation for a resolved menu action. */
export function createMenuEntryActionViewItem(
  action: IAction,
  options: MenuEntryActionViewItemOptions = {},
): ActionViewItem {
  if (action instanceof Separator) {
    return new SeparatorActionViewItem(action);
  }
  if (action instanceof SubmenuAction) {
    return new SubmenuActionViewItem(action, options);
  }
  return new MenuEntryActionViewItem(action, options);
}

export interface ActionMenuOptions {
  readonly actions: readonly IAction[];
  readonly ownerDocument: Document;
  readonly onDidSelect: () => void;
  readonly onDidRequestClose?: () => void;
  readonly getKeybinding?: (
    action: IAction,
  ) => ResolvedKeybinding | undefined;
  readonly layer?: number;
}

/** Browser menu that presents resolved actions and nested submenus. */
export class ActionMenu extends DisposableOwner {
  readonly element: HTMLDivElement;
  readonly #submenus: SubmenuActionViewItem[] = [];

  constructor(options: ActionMenuOptions) {
    super();
    const element = options.ownerDocument.createElement("div");
    this.element = element;
    this.defer(() => element.remove());
    element.className = "zeta-menu zeta-action-menu";
    element.setAttribute("role", "menu");

    for (const action of options.actions) {
      const item = this.own(
        createMenuEntryActionViewItem(action, {
          onDidSelect: options.onDidSelect,
          submenuLayer: (options.layer ?? 20) + 1,
          keybinding: options.getKeybinding?.(action),
          getKeybinding: options.getKeybinding,
        }),
      );
      if (item instanceof SubmenuActionViewItem) {
        this.#submenus.push(item);
      }
      const container = options.ownerDocument.createElement("div");
      container.className = "zeta-action-view-item";
      container.dataset.actionId = action.id;
      container.setAttribute("role", "presentation");
      element.append(container);
      item.render(container);
      const button = container.querySelector("button");
      if (button) {
        if (action.checked === undefined) {
          button.setAttribute("role", "menuitem");
        } else {
          button.setAttribute("role", "menuitemcheckbox");
          button.setAttribute("aria-checked", String(action.checked));
          button.removeAttribute("aria-pressed");
        }
      }
    }
    this.own(addDisposableListener(element, "keydown", (event) => {
      if (event.isComposing) return;
      let handled = true;
      switch (event.key) {
        case "ArrowDown":
          moveFocus(
            element,
            FocusNavigationDirection.Forward,
            FocusNavigationBoundary.Wrap,
          );
          break;
        case "ArrowUp":
          moveFocus(
            element,
            FocusNavigationDirection.Backward,
            FocusNavigationBoundary.Wrap,
          );
          break;
        case "Home":
          focusFirst(element);
          break;
        case "End":
          focusLast(element);
          break;
        case "ArrowRight":
          {
            const activeElement = element.ownerDocument.activeElement;
            const submenu = this.#submenus.find((item) =>
              item.ownsTrigger(activeElement)
            );
            if (submenu) submenu.openFromKeyboard();
            else handled = false;
          }
          break;
        case "ArrowLeft":
          options.onDidRequestClose?.();
          handled = options.onDidRequestClose !== undefined;
          break;
        default:
          handled = false;
      }
      if (!handled) return;
      event.preventDefault();
      event.stopPropagation();
    }));
  }

  focusFirst(): void {
    focusFirst(this.element);
  }

  contains(target: Node): boolean {
    return this.element.contains(target) ||
      this.#submenus.some((submenu) => submenu.contains(target));
  }
}
