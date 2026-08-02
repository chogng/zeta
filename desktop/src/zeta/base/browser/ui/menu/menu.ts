import { type IAction, Separator, SubmenuAction } from "../../../common/actions.js";
import { addDisposableListener, isNode } from "../../dom.js";
import { FocusNavigationBoundary, FocusNavigationDirection, focusFirst, focusLast, moveFocus } from "../../focus.js";
import type { ResolvedKeybinding } from "../../../common/keybindings.js";
import { DisposableOwner } from "../../../common/lifecycle.js";
import { lxiconsLibrary } from "../../../common/lxiconsLibrary.js";
import { ActionViewItem, ButtonActionViewItem, SeparatorActionViewItem } from "../actionbar/actionViewItems.js";
import { AnchorAxisAlignment, AnchorPosition, ContextView, ContextViewFocusRestore } from "../contextview/contextview.js";
import { appendIcon } from "../icon/icon.js";
import { KeybindingLabel } from "../keybindinglabel/keybindinglabel.js";

interface MenuActionViewItemOptions {
  readonly onDidSelect?: () => void;
  readonly submenuLayer?: number;
  readonly contextViewContainer?: HTMLElement;
  readonly keybinding?: ResolvedKeybinding;
  readonly getKeybinding?: (
    action: IAction,
  ) => ResolvedKeybinding | undefined;
}

function prependMenuLeadingSlot(
  button: HTMLButtonElement,
  checked: boolean | undefined,
): void {
  const slot = button.ownerDocument.createElement("span");
  slot.className = "zeta-menu-leading-slot";
  slot.setAttribute("aria-hidden", "true");
  const icon = button.querySelector<SVGElement>(":scope > .zeta-icon");
  if (icon) slot.append(icon);
  else if (checked !== undefined) {
    slot.classList.add("zeta-menu-leading-check");
    appendIcon(lxiconsLibrary.check, slot);
  }
  button.prepend(slot);
}

/** Button view item for an action presented inside a menu. */
class MenuActionViewItem extends ButtonActionViewItem {
  private readonly onDidSelect: (() => void) | undefined;
  private readonly keybinding: ResolvedKeybinding | undefined;

  constructor(
    action: IAction,
    options: MenuActionViewItemOptions = {},
  ) {
    super(action);
    this.onDidSelect = options.onDidSelect;
    this.keybinding = options.keybinding;
  }

  override render(container: HTMLElement): void {
    super.render(container);
    prependMenuLeadingSlot(this.button.element, this.action.checked);
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
    if (!this.keybinding) return;
    const label = this.own(new KeybindingLabel({
      keybinding: this.keybinding,
      ownerDocument: container.ownerDocument,
    }));
    label.element.classList.add("zeta-menu-keybinding");
    this.button.element.append(label.element);
  }

  protected override runAction(): unknown {
    try {
      return super.runAction();
    } finally {
      this.onDidSelect?.();
    }
  }
}

/** Menu view item that owns the nested Menu for a submenu action. */
class SubmenuMenuActionViewItem extends ButtonActionViewItem {
  private readonly submenuAction: SubmenuAction;
  private readonly onDidSelect: (() => void) | undefined;
  private readonly submenuLayer: number;
  private readonly contextViewContainer: HTMLElement | undefined;
  private readonly getKeybinding:
    | ((action: IAction) => ResolvedKeybinding | undefined)
    | undefined;
  private contextView: ContextView | undefined;
  private menu: Menu | undefined;
  private open = false;

  constructor(
    action: SubmenuAction,
    options: MenuActionViewItemOptions = {},
  ) {
    super(action);
    this.submenuAction = action;
    this.onDidSelect = options.onDidSelect;
    this.submenuLayer = options.submenuLayer ?? 20;
    this.contextViewContainer = options.contextViewContainer;
    this.getKeybinding = options.getKeybinding;
  }

  override render(container: HTMLElement): void {
    super.render(container);
    prependMenuLeadingSlot(this.button.element, undefined);
    const ownerDocument = container.ownerDocument;
    if (
      this.contextViewContainer &&
      this.contextViewContainer.ownerDocument !== ownerDocument
    ) {
      throw new Error("Context view container belongs to another document");
    }
    this.contextView = this.own(new ContextView(
      this.contextViewContainer ?? ownerDocument.body,
    ));
    this.menu = this.own(new Menu({
      actions: this.submenuAction.actions,
      ownerDocument,
      contextViewContainer: this.contextViewContainer,
      layer: this.submenuLayer,
      getKeybinding: this.getKeybinding,
      onDidSelect: () => {
        this.hide();
        this.onDidSelect?.();
      },
      onDidRequestClose: () => {
        this.hide();
        this.button.element.focus();
      },
    }));
    this.own(this.contextView.onDidHide(() => {
      this.open = false;
      this.button.element.setAttribute("aria-expanded", "false");
    }));
    this.button.element.setAttribute("role", "menuitem");
    this.button.element.setAttribute("aria-haspopup", "menu");
    this.button.element.setAttribute("aria-expanded", "false");
    const indicator = ownerDocument.createElement("span");
    indicator.className = "zeta-submenu-indicator";
    appendIcon(lxiconsLibrary.submenuIndicator, indicator);
    this.button.element.append(indicator);
  }

  protected override runAction(): void {
    if (this.open) {
      this.hide();
      return;
    }
    if (!this.contextView || !this.menu) return;
    this.contextView.show({
      anchor: this.button.element,
      content: this.menu.element,
      anchorAxisAlignment: AnchorAxisAlignment.Horizontal,
      anchorPosition: AnchorPosition.Below,
      gap: 2,
      presentation: "menu",
      layer: this.submenuLayer,
      focusRestore: ContextViewFocusRestore.Previous,
      isTargetWithin: (target) => this.menu?.contains(target) ?? false,
    });
    this.open = true;
    this.button.element.setAttribute("aria-expanded", "true");
    this.menu.focusFirst();
  }

  private hide(): void {
    if (!this.open) return;
    this.open = false;
    this.contextView?.hide();
  }

  contains(target: Node): boolean {
    return this.contextView?.element.contains(target) === true ||
      this.menu?.contains(target) === true;
  }

  ownsTrigger(target: Element | null): boolean {
    return target === this.button.element;
  }

  openFromKeyboard(): void {
    if (!this.open) this.runAction();
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
  readonly contextViewContainer?: HTMLElement;
  readonly onDidSelect?: () => void;
  readonly onDidRequestClose?: () => void;
  readonly getKeybinding?: (
    action: IAction,
  ) => ResolvedKeybinding | undefined;
  readonly layer?: number;
}

interface MenuEntry {
  readonly action: IAction;
  readonly container: HTMLElement;
  readonly item: ActionViewItem;
}

/** Keyboard-focusable action menu with shared nested-submenu behavior. */
export class Menu extends DisposableOwner {
  readonly element: HTMLDivElement;
  private readonly submenus: SubmenuMenuActionViewItem[] = [];
  private readonly entries: MenuEntry[] = [];
  private focusedEntry: MenuEntry | undefined;

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
          contextViewContainer: options.contextViewContainer,
          keybinding: options.getKeybinding?.(action),
          getKeybinding: options.getKeybinding,
        }),
      );
      if (item instanceof SubmenuMenuActionViewItem) {
        this.submenus.push(item);
      }
      const container = ownerDocument.createElement("div");
      container.className = "zeta-action-view-item";
      container.dataset.actionId = action.id;
      container.setAttribute("role", "presentation");
      element.append(container);
      item.render(container);
      this.entries.push({ action, container, item });
    }
    this.own(addDisposableListener(element, "focusin", (event) => {
      const entry = this.findEntry(event.target);
      if (entry?.action.enabled) this.setFocusedEntry(entry);
    }));
    this.own(addDisposableListener(element, "focusout", (event) => {
      if (isNode(event.relatedTarget) && this.contains(event.relatedTarget)) return;
      this.setFocusedEntry(undefined);
    }));
    this.own(addDisposableListener(element, "mouseover", (event) => {
      const entry = this.findEntry(event.target);
      this.setFocusedEntry(entry?.action.enabled ? entry : undefined, true);
    }));
    this.own(addDisposableListener(element, "mouseout", (event) => {
      if (isNode(event.relatedTarget) && this.contains(event.relatedTarget)) return;
      this.setFocusedEntry(undefined);
    }));
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
            const submenu = this.submenus.find((item) =>
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
      this.submenus.some((submenu) => submenu.contains(target));
  }

  private findEntry(target: EventTarget | null): MenuEntry | undefined {
    if (!isNode(target)) return undefined;
    const targetElement = target.nodeType === 1
      ? target as Element
      : target.parentElement;
    const container = targetElement?.closest<HTMLElement>(
      ".zeta-action-view-item",
    );
    if (container?.parentElement !== this.element) return undefined;
    return this.entries.find((entry) => entry.container === container);
  }

  private setFocusedEntry(entry: MenuEntry | undefined, focus = false): void {
    if (entry !== this.focusedEntry) {
      this.focusedEntry?.container.classList.remove("focused");
      this.focusedEntry = entry;
      entry?.container.classList.add("focused");
    }
    if (
      focus &&
      entry &&
      !entry.container.contains(this.element.ownerDocument.activeElement)
    ) {
      entry.item.focus();
    }
  }
}
