import { addDisposableListener } from "../../../../base/browser/dom.js";
import { focusPreservingScroll } from "../../../../base/browser/focus.js";
import { Button } from "../../../../base/browser/ui/button/button.js";
import { SubmenuAction } from "../../../../base/common/actions.js";
import {
  DisposableOwner,
  ResettableDisposableGroup,
  type IDisposable,
} from "../../../../base/common/lifecycle.js";
import {
  MenuId,
} from "../../../../platform/actions/common/actions.js";
import type {
  IMenu,
  IMenuService,
} from "../../../../platform/actions/common/menuService.js";
import type {
  IContextMenuService,
} from "../../../../platform/contextview/browser/contextMenu.js";

/** Host-selected menubar presentation owned by the titlebar. */
export interface IMenubarControl extends IDisposable {
  readonly element: HTMLElement | undefined;
}

/** HTML menubar used by web, Windows, and Linux workbenches. */
export class BrowserMenubarControl extends DisposableOwner
  implements IMenubarControl {
  readonly #menu: IMenu & Disposable;
  readonly #items = this.own(new ResettableDisposableGroup());
  readonly #contextMenuService: IContextMenuService;
  readonly #buttons: Array<{
    readonly action: SubmenuAction;
    readonly element: HTMLButtonElement;
  }> = [];
  #activeButton: HTMLButtonElement | undefined;

  readonly element: HTMLElement;

  constructor(
    menuService: IMenuService,
    contextMenuService: IContextMenuService,
    ownerDocument: Document,
  ) {
    super();
    this.#contextMenuService = contextMenuService;
    this.element = ownerDocument.createElement("nav");
    this.element.className = "zeta-menubar";
    this.element.setAttribute("aria-label", "Application menu");
    this.element.setAttribute("role", "menubar");
    this.defer(() => this.element.remove());

    this.#menu = this.own(menuService.createMenu(MenuId.MenubarMainMenu));
    this.own(this.#menu.onDidChange(() => this.#render()));
    this.own(addDisposableListener(
      this.element,
      "keydown",
      (event: KeyboardEvent) => this.#onKeyDown(event),
    ));
    this.defer(() => {
      if (this.#activeButton) {
        this.#contextMenuService.hideContextMenu();
      }
    });
    this.#render();
  }

  #render(): void {
    if (this.#activeButton) {
      this.#contextMenuService.hideContextMenu();
    }
    this.#items.clear();
    this.#buttons.length = 0;
    this.element.replaceChildren();

    const actions = this.#menu.getActions()
      .flatMap(([, groupActions]) => groupActions)
      .filter((action): action is SubmenuAction =>
        action instanceof SubmenuAction
    );
    for (const action of actions) {
      const button = this.#items.add(new Button({
        label: action.label,
        ownerDocument: this.element.ownerDocument,
      }));
      this.#items.add(addDisposableListener(
        button.element,
        "click",
        () => this.#toggleMenu(action, button.element),
      ));
      button.element.classList.add("zeta-menubar-item");
      button.element.setAttribute("aria-haspopup", "menu");
      button.element.setAttribute("aria-expanded", "false");
      button.element.setAttribute("role", "menuitem");
      button.element.tabIndex = this.#buttons.length === 0 ? 0 : -1;
      this.#items.add(addDisposableListener(
        button.element,
        "focus",
        () => this.#setTabStop(button.element),
      ));
      this.#items.add(addDisposableListener(
        button.element,
        "pointerenter",
        () => {
          if (
            this.#activeButton &&
            this.#activeButton !== button.element
          ) {
            this.#showMenu(action, button.element);
          }
        },
      ));
      this.#buttons.push({
        action,
        element: button.element,
      });
      this.element.append(button.element);
    }
  }

  #toggleMenu(
    action: SubmenuAction,
    button: HTMLButtonElement,
  ): void {
    if (this.#activeButton === button) {
      this.#contextMenuService.hideContextMenu();
      return;
    }
    this.#showMenu(action, button);
  }

  #showMenu(
    action: SubmenuAction,
    button: HTMLButtonElement,
  ): void {
    this.#contextMenuService.hideContextMenu();
    this.#activeButton = button;
    button.setAttribute("aria-expanded", "true");
    button.classList.add("active");
    this.#contextMenuService.showContextMenu({
      anchor: button,
      actions: action.actions,
      onHide: () => {
        button.setAttribute("aria-expanded", "false");
        button.classList.remove("active");
        if (this.#activeButton === button) {
          this.#activeButton = undefined;
        }
      },
    });
  }

  #onKeyDown(event: KeyboardEvent): void {
    if (
      event.isComposing ||
      event.altKey ||
      event.ctrlKey ||
      event.metaKey
    ) {
      return;
    }

    switch (event.key) {
      case "ArrowLeft":
        this.#moveFocus(-1);
        break;
      case "ArrowRight":
        this.#moveFocus(1);
        break;
      case "Home":
        this.#focusButton(0);
        break;
      case "End":
        this.#focusButton(this.#buttons.length - 1);
        break;
      case "ArrowDown": {
        const item = this.#focusedItem();
        if (item) this.#showMenu(item.action, item.element);
        break;
      }
      case "Escape":
        if (!this.#activeButton) return;
        this.#contextMenuService.hideContextMenu();
        break;
      default:
        return;
    }
    event.preventDefault();
    event.stopPropagation();
  }

  #moveFocus(delta: -1 | 1): void {
    if (this.#buttons.length === 0) return;
    const current = this.#focusedIndex();
    const next = current < 0
      ? delta > 0 ? 0 : this.#buttons.length - 1
      : (current + delta + this.#buttons.length) % this.#buttons.length;
    this.#focusButton(next);
  }

  #focusButton(index: number): void {
    const item = this.#buttons[index];
    if (!item) return;
    this.#setTabStop(item.element);
    focusPreservingScroll(item.element);
  }

  #setTabStop(button: HTMLButtonElement): void {
    for (const item of this.#buttons) {
      item.element.tabIndex = item.element === button ? 0 : -1;
    }
  }

  #focusedItem(): {
    readonly action: SubmenuAction;
    readonly element: HTMLButtonElement;
  } | undefined {
    return this.#buttons[this.#focusedIndex()];
  }

  #focusedIndex(): number {
    const activeElement = this.element.ownerDocument.activeElement;
    return this.#buttons.findIndex(({ element }) => element === activeElement);
  }
}
