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
  readonly ownerDocument: Document;
}

/** Creates the titlebar implementation selected by the current host. */
export type TitlebarPartFactory = (
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
    options: ITitlebarPartFactoryOptions,
    menubar: IMenubarControl,
  ) {
    super("titlebar", options.ownerDocument);
    this.menubar = this.own(menubar);
    this.leftActions = this.own(
      new MenuWorkbenchToolBar(
        options.menuService,
        options.contextMenuService,
        MenuId.TitleBarLeft,
        options.ownerDocument,
        { presentation: "inherit-foreground" },
      ),
    );
    this.actions = this.own(
      new MenuWorkbenchToolBar(
        options.menuService,
        options.contextMenuService,
        MenuId.TitleBar,
        options.ownerDocument,
        { presentation: "inherit-foreground" },
      ),
    );
    const leftActionsElement = h(options.ownerDocument, "div");
    leftActionsElement.className = "zeta-titlebar-left-actions zeta-titlebar-interactive-region";
    leftActionsElement.append(this.leftActions.element);
    this.titleElement.append(leftActionsElement);
    if (this.menubar.element) {
      this.menubar.element.classList.add("zeta-titlebar-interactive-region");
      this.titleElement.append(this.menubar.element);
    }
    const actionsElement = h(options.ownerDocument, "div");
    actionsElement.className = "zeta-titlebar-actions zeta-titlebar-interactive-region";
    actionsElement.append(this.actions.element);
    this.contentElement.append(actionsElement);
  }
}

/** Creates the titlebar used by a regular web workbench. */
export const createBrowserTitlebarPart: TitlebarPartFactory = (options) =>
  new BrowserTitlebarPart(
    options,
    new BrowserMenubarControl(
      options.menuService,
      options.contextMenuService,
      options.ownerDocument,
    ),
  );
