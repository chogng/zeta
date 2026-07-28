import { Emitter } from "../../../base/common/event.js";
import { isNode } from "../../../base/browser/dom.js";
import {
  DisposableOwner,
  DisposableSlot,
} from "../../../base/common/lifecycle.js";
import type { IRectangle } from "../../../base/common/layout.js";
import {
  AnchorPosition,
  ContextView,
  ContextViewFocusRestore,
} from "../../../base/browser/ui/contextview/contextview.js";
import {
  ActionMenu,
} from "../../actions/browser/menuEntryActionViewItem.js";
import type { IMenuService } from "../../actions/common/menuService.js";
import type {
  IKeybindingService,
} from "../../keybinding/common/keybinding.js";
import {
  type ContextMenuAnchor,
  type ContextMenuOptions,
  type IContextMenuService,
  resolveContextMenuActions,
} from "./contextMenu.js";

/** HTML implementation used by web, Windows, and Linux workbenches. */
export class BrowserContextMenuService extends DisposableOwner
  implements IContextMenuService {
  readonly #onDidShowContextMenu = this.own(new Emitter<void>());
  readonly #onDidHideContextMenu = this.own(new Emitter<void>());
  readonly #activeMenu = this.own(new DisposableSlot<ActionMenu>());
  readonly #contextView: ContextView;
  readonly #menuService: IMenuService;
  readonly #keybindingService: IKeybindingService;
  #onHide: ((didCancel: boolean) => void) | undefined;
  #didSelect = false;
  #active = false;
  #shown = false;

  readonly onDidShowContextMenu = this.#onDidShowContextMenu.event;
  readonly onDidHideContextMenu = this.#onDidHideContextMenu.event;

  constructor(
    menuService: IMenuService,
    keybindingService: IKeybindingService,
    ownerDocument: Document,
  ) {
    super();
    this.#menuService = menuService;
    this.#keybindingService = keybindingService;
    this.#contextView = this.own(new ContextView(ownerDocument));
    this.defer(() => this.hideContextMenu());
  }

  showContextMenu(options: ContextMenuOptions): void {
    this.hideContextMenu();
    const actions = resolveContextMenuActions(options, this.#menuService);
    if (actions.length === 0) {
      options.onHide?.(true);
      return;
    }

    this.#didSelect = false;
    this.#active = true;
    this.#shown = false;
    this.#onHide = options.onHide;
    const menu = new ActionMenu({
      actions,
      ownerDocument: this.#contextView.element.ownerDocument,
      layer: 10,
      getKeybinding: (action) =>
        this.#keybindingService.lookupKeybinding(action.id),
      onDidSelect: () => {
        this.#didSelect = true;
        this.#contextView.hide();
      },
    });
    this.#activeMenu.replace(menu);
    const shown = this.#contextView.show({
      anchor: toContextViewAnchor(options.anchor),
      content: menu.element,
      anchorPosition: AnchorPosition.Below,
      focusRestore: ContextViewFocusRestore.Previous,
      layer: 10,
      isTargetWithin: (target) => menu.contains(target),
      onHide: () => this.#didHide(),
    });
    if (!shown) {
      this.#didHide();
      return;
    }

    this.#shown = true;
    menu.focusFirst();
    this.#onDidShowContextMenu.fire();
  }

  hideContextMenu(): void {
    if (this.#contextView.visible) {
      this.#contextView.hide();
    } else if (this.#activeMenu.value) {
      this.#didHide();
    }
  }

  #didHide(): void {
    if (!this.#active) return;
    this.#active = false;
    const onHide = this.#onHide;
    const didCancel = !this.#didSelect;
    const shown = this.#shown;
    this.#onHide = undefined;
    this.#didSelect = false;
    this.#shown = false;
    this.#activeMenu.clear();
    onHide?.(didCancel);
    if (shown) this.#onDidHideContextMenu.fire();
  }
}

function toContextViewAnchor(
  anchor: ContextMenuAnchor,
): Element | IRectangle {
  if (!isNode(anchor)) {
    return {
      left: anchor.x,
      top: anchor.y,
      width: 0,
      height: 0,
    };
  }
  return anchor;
}
