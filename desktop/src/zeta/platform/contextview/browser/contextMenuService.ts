import { Emitter } from "../../../base/common/event.js";
import { isNode } from "../../../base/browser/dom.js";
import {
  DisposableOwner,
  DisposableSlot,
} from "../../../base/common/lifecycle.js";
import type { IRectangle } from "../../../base/common/layout.js";
import {
  AnchorPosition,
  ContextViewFocusRestore,
} from "../../../base/browser/ui/contextview/contextview.js";
import { Menu } from "../../../base/browser/ui/menu/menu.js";
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
import type { IContextViewService } from "./contextView.js";

/** HTML implementation used by web, Windows, and Linux workbenches. */
export class BrowserContextMenuService extends DisposableOwner
  implements IContextMenuService {
  private readonly _onDidShowContextMenu = this.own(new Emitter<void>());
  private readonly _onDidHideContextMenu = this.own(new Emitter<void>());
  private readonly activeMenu = this.own(new DisposableSlot<Menu>());
  private readonly contextViewService: IContextViewService;
  private readonly menuService: IMenuService;
  private readonly keybindingService: IKeybindingService;
  private onHide: ((didCancel: boolean) => void) | undefined;
  private didSelect = false;
  private active = false;
  private shown = false;

  readonly onDidShowContextMenu = this._onDidShowContextMenu.event;
  readonly onDidHideContextMenu = this._onDidHideContextMenu.event;

  constructor(
    menuService: IMenuService,
    keybindingService: IKeybindingService,
    contextViewService: IContextViewService,
  ) {
    super();
    this.menuService = menuService;
    this.keybindingService = keybindingService;
    this.contextViewService = contextViewService;
    this.defer(() => this.hideContextMenu());
  }

  showContextMenu(options: ContextMenuOptions): void {
    this.hideContextMenu();
    const actions = resolveContextMenuActions(options, this.menuService);
    if (actions.length === 0) {
      options.onHide?.(true);
      return;
    }

    this.didSelect = false;
    this.active = true;
    this.shown = false;
    this.onHide = options.onHide;
    const menu = new Menu({
      actions,
      ownerDocument: this.contextViewService.container.ownerDocument,
      contextViewContainer: this.contextViewService.container,
      layer: 10,
      getKeybinding: (action) =>
        this.keybindingService.lookupKeybinding(action.id),
      onDidSelect: () => {
        this.didSelect = true;
        this.contextViewService.hide();
      },
    });
    this.activeMenu.replace(menu);
    const shown = this.contextViewService.show({
      anchor: toContextViewAnchor(options.anchor),
      content: menu.element,
      anchorPosition: AnchorPosition.Below,
      focusRestore: ContextViewFocusRestore.Previous,
      layer: 10,
      isTargetWithin: (target) => menu.contains(target),
      onHide: () => this.didHide(),
    });
    if (!shown) {
      this.didHide();
      return;
    }

    this.shown = true;
    menu.focusFirst();
    this._onDidShowContextMenu.fire();
  }

  hideContextMenu(): void {
    if (!this.active) return;
    this.contextViewService.hide();
  }

  private didHide(): void {
    if (!this.active) return;
    this.active = false;
    const onHide = this.onHide;
    const didCancel = !this.didSelect;
    const shown = this.shown;
    this.onHide = undefined;
    this.didSelect = false;
    this.shown = false;
    this.activeMenu.clear();
    onHide?.(didCancel);
    if (shown) this._onDidHideContextMenu.fire();
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
