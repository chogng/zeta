import {
  WorkbenchToolBar,
} from "../../../../platform/actions/browser/toolbar.js";
import {
  MenuId,
} from "../../../../platform/actions/common/actions.js";
import type {
  IMenuService,
} from "../../../../platform/actions/common/menuService.js";
import type {
  IContextMenuService,
} from "../../../../platform/contextview/browser/contextMenu.js";
import { WorkbenchPart } from "../../part.js";
import {
  BrowserMenubarControl,
  type IMenubarControl,
} from "./menubarControl.js";

/** Inputs shared by web and Electron titlebar factories. */
export interface ITitlebarPartFactoryOptions {
  readonly menuService: IMenuService;
  readonly contextMenuService: IContextMenuService;
  readonly ownerDocument: Document;
  readonly title: string;
}

/** Creates the titlebar implementation selected by the current host. */
export type TitlebarPartFactory = (
  options: ITitlebarPartFactoryOptions,
) => BrowserTitlebarPart;

/** The host-neutral workbench title area and its actions. */
export class BrowserTitlebarPart extends WorkbenchPart {
  readonly #label: HTMLHeadingElement;
  readonly #menubar: IMenubarControl;
  readonly #actions: WorkbenchToolBar;

  constructor(
    options: ITitlebarPartFactoryOptions,
    menubar: IMenubarControl,
  ) {
    super("titlebar", options.ownerDocument);
    this.#label = options.ownerDocument.createElement("h1");
    this.#label.className = "zeta-titlebar-label";
    this.#label.textContent = options.title;
    this.#menubar = this.own(menubar);
    this.#actions = this.own(
      new WorkbenchToolBar(
        options.menuService,
        MenuId.TitleBar,
        options.ownerDocument,
      ),
    );
    this.titleElement.append(this.#label);
    if (this.#menubar.element) {
      this.titleElement.append(this.#menubar.element);
    }
    this.contentElement.append(this.#actions.element);
  }

  setTitle(title: string): void { this.#label.textContent = title; }
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
