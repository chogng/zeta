import type { Event } from "../../../../base/common/event.js";
import {
  DisposableOwner,
  type IDisposable,
} from "../../../../base/common/lifecycle.js";
import type {
  IMenuService,
} from "../../../../platform/actions/common/menuService.js";
import {
  type ContextMenuOptions,
  type IContextMenuService,
} from "../../../../platform/contextview/browser/contextMenu.js";
import type {
  IKeybindingService,
} from "../../../../platform/keybinding/common/keybinding.js";
import type {
  IContextViewService,
} from "../../../../platform/contextview/browser/contextView.js";

/** Product services required to construct a context menu for one window. */
export interface WorkbenchContextMenuServiceOptions {
  readonly menuService: IMenuService;
  readonly keybindingService: IKeybindingService;
  readonly contextViewService: IContextViewService;
}

/** Creates the host-specific product context menu service for one workbench. */
export type WorkbenchContextMenuServiceFactory = (
  options: WorkbenchContextMenuServiceOptions,
) => IContextMenuService & IDisposable;

/**
 * Workbench-owned context menu facade.
 *
 * Host entry points choose the rendering implementation while consumers use
 * one stable service contract. The facade owns that implementation for the
 * lifetime of the workbench window.
 */
export class WorkbenchContextMenuService
  extends DisposableOwner
  implements IContextMenuService {
  readonly #implementation: IContextMenuService;

  readonly onDidShowContextMenu: Event<void>;
  readonly onDidHideContextMenu: Event<void>;

  constructor(implementation: IContextMenuService & IDisposable) {
    super();
    this.#implementation = this.own(implementation);
    this.onDidShowContextMenu = implementation.onDidShowContextMenu;
    this.onDidHideContextMenu = implementation.onDidHideContextMenu;
  }

  showContextMenu(options: ContextMenuOptions): void {
    this.#implementation.showContextMenu(options);
  }

  hideContextMenu(): void {
    this.#implementation.hideContextMenu();
  }
}
