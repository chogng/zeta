import {
  type IAction,
  Separator,
  SubmenuAction,
} from "../../../common/actions.js";
import { addDisposableListener } from "../../dom.js";
import {
  FocusNavigationBoundary,
  FocusNavigationDirection,
  focusFirst,
  focusLast,
  moveFocus,
} from "../../focus.js";
import type { ResolvedKeybinding } from "../../../common/keybindings.js";
import { DisposableOwner } from "../../../common/lifecycle.js";
import { LxIcon } from "../../../common/lxicons.js";
import {
  ActionViewItem,
  ButtonActionViewItem,
  SeparatorActionViewItem,
} from "../actionbar/actionViewItems.js";
import {
  AnchorAxisAlignment,
  AnchorPosition,
  ContextView,
  ContextViewFocusRestore,
} from "../contextview/contextview.js";
import { appendIcon } from "../icon/icon.js";
import {
  KeybindingLabel,
} from "../keybindinglabel/keybindinglabel.js";

interface MenuActionViewItemOptions {
  readonly onDidSelect?: () => void;
  readonly submenuLayer?: number;
  readonly keybinding?: ResolvedKeybinding;
  readonly getKeybinding?: (
    action: IAction,
  ) => ResolvedKeybinding | undefined;
}

/** Button view item for an action presented inside a menu. */
class MenuActionViewItem extends ButtonActionViewItem {
  readonly #onDidSelect: (() => void) | undefined;
  readonly #keybinding: ResolvedKeybinding | undefined;

  constructor(
    action: IAction,
    options: MenuActionViewItemOptions = {},
  ) {
    super(action);
    this.#onDidSelect = options.onDidSelect;
    this.#keybinding = options.keybinding;
  }

  override render(container: HTMLElement): void {
    super.render(container);
    if (this.action.checked === undefined) {
      this.button.element.setAttribute("role", "menuitem");
    } else {
      this.button.element.setAttribute("role", "menuitemcheckbox");
      this.button.element.setAttribute(
        "aria-checked",
        String(this.action.checked),
      );
      this.button.element.removeAttribute("aria-pressed");
    }
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

/** Menu view item that owns the nested Menu for a submenu action. */
class SubmenuMenuActionViewItem extends ButtonActionViewItem {
  readonly #submenuAction: SubmenuAction;
  readonly #onDidSelect: (() => void) | undefined;
  readonly #submenuLayer: number;
  readonly #getKeybinding:
    | ((action: IAction) => ResolvedKeybinding | undefined)
    | undefined;
  #contextView: ContextView | undefined;
  #menu: Menu | undefined;
  #open = false;

  constructor(
    action: SubmenuAction,
    options: MenuActionViewItemOptions = {},
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
    this.#menu = this.own(new Menu({
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
    this.button.element.setAttribute("role", "menuitem");
    this.button.element.setAttribute("aria-haspopup", "menu");
    this.button.element.setAttribute("aria-expanded", "false");
    const indicator = ownerDocument.createElement("span");
    indicator.className = "zeta-submenu-indicator";
    appendIcon(LxIcon.submenuIndicator, indicator);
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

/** Creates the menu representation for an action. */
function createMenuActionViewItem(
  action: IAction,
  options: MenuActionViewItemOptions = {},
): ActionViewItem {
  if (action instanceof Separator) {
    return new SeparatorActionViewItem(action);
  }
  if (action instanceof SubmenuAction) {
    return new SubmenuMenuActionViewItem(action, options);
  }
  return new MenuActionViewItem(action, options);
}

export interface MenuOptions {
  readonly actions: readonly IAction[];
  readonly ownerDocument?: Document;
  readonly onDidSelect?: () => void;
  readonly onDidRequestClose?: () => void;
  readonly getKeybinding?: (
    action: IAction,
  ) => ResolvedKeybinding | undefined;
  readonly layer?: number;
}

/** Keyboard-focusable action menu with shared nested-submenu behavior. */
export class Menu extends DisposableOwner {
  readonly element: HTMLDivElement;
  readonly #submenus: SubmenuMenuActionViewItem[] = [];

  constructor(options: MenuOptions) {
    super();
    const ownerDocument = options.ownerDocument ?? document;
    const element = ownerDocument.createElement("div");
    this.element = element;
    this.defer(() => element.remove());
    element.className = "zeta-menu";
    element.setAttribute("role", "menu");

    for (const action of options.actions) {
      const item = this.own(
        createMenuActionViewItem(action, {
          onDidSelect: options.onDidSelect,
          submenuLayer: (options.layer ?? 20) + 1,
          keybinding: options.getKeybinding?.(action),
          getKeybinding: options.getKeybinding,
        }),
      );
      if (item instanceof SubmenuMenuActionViewItem) {
        this.#submenus.push(item);
      }
      const container = ownerDocument.createElement("div");
      container.className = "zeta-action-view-item";
      container.dataset.actionId = action.id;
      container.setAttribute("role", "presentation");
      element.append(container);
      item.render(container);
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
