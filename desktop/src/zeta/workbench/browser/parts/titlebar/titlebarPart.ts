import "./titlebarpart.css";
import { MenuWorkbenchToolBar } from "../../../../platform/actions/browser/toolbar.js";
import { MenuId } from "../../../../platform/actions/common/actions.js";
import type { IMenuService } from "../../../../platform/actions/common/menuService.js";
import type { IContextMenuService } from "../../../../platform/contextview/browser/contextMenu.js";
import { WorkbenchPart } from "../../part.js";
import { WorkbenchWindowBarHeight } from "../workbenchPartDimensions.js";
import { BrowserMenubarControl, type IMenubarControl } from "./menubarControl.js";
import { h } from "../../../../base/browser/dom.js";

/** Inputs shared by web and Electron titlebar factories. */
export interface ITitlebarPartFactoryOptions {
  readonly menuService: IMenuService;
  readonly contextMenuService: IContextMenuService;
}

/** Creates the titlebar implementation selected by the current host. */
export type TitlebarPartFactory = (
  container: HTMLElement,
  options: ITitlebarPartFactoryOptions,
) => BrowserTitlebarPart;

/** The host-neutral workbench title area and its actions. */
export class BrowserTitlebarPart extends WorkbenchPart {
  private readonly menubar: IMenubarControl;
  private readonly leftActions: MenuWorkbenchToolBar;
  private readonly actions: MenuWorkbenchToolBar;

  override get minimumHeight(): number { return WorkbenchWindowBarHeight; }
  override get maximumHeight(): number { return WorkbenchWindowBarHeight; }

  constructor(
    container: HTMLElement,
    options: ITitlebarPartFactoryOptions,
    menubar: IMenubarControl,
  ) {
    super(container, "titlebar");
    const ownerDocument = container.ownerDocument;
    this.menubar = this.own(menubar);
    const leftActionsElement = h(ownerDocument, "div");
    leftActionsElement.className = "zeta-titlebar-left-actions zeta-titlebar-interactive-region";
    this.titleElement.append(leftActionsElement);
    this.leftActions = this.own(
      new MenuWorkbenchToolBar(
        leftActionsElement,
        options.menuService,
        options.contextMenuService,
        MenuId.TitleBarLeft,
        { presentation: "inherit-foreground" },
      ),
    );
    const actionsElement = h(ownerDocument, "div");
    actionsElement.className = "zeta-titlebar-actions zeta-titlebar-interactive-region";
    this.contentElement.append(actionsElement);
    this.actions = this.own(
      new MenuWorkbenchToolBar(
        actionsElement,
        options.menuService,
        options.contextMenuService,
        MenuId.TitleBar,
        { presentation: "inherit-foreground" },
      ),
    );
    if (this.menubar.element) {
      this.menubar.element.classList.add("zeta-titlebar-interactive-region");
      this.titleElement.append(this.menubar.element);
    }
  }
}

/** Creates the titlebar used by a regular web workbench. */
export const createBrowserTitlebarPart: TitlebarPartFactory = (container, options) =>
  new BrowserTitlebarPart(
    container,
    options,
    new BrowserMenubarControl(
      container,
      options.menuService,
      options.contextMenuService,
    ),
  );
