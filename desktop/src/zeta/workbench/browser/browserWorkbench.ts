import { installBaseUiStyles } from "../../base/browser/ui/index.js";
import type { IDisposable } from "../../base/common/lifecycle.js";
import type {
  ProductConfiguration,
} from "../../product/common/product.js";
import type {
  ZetaRendererApi,
} from "../../platform/app-server/common/renderer-api.js";
import {
  UNKNOWN_EMPTY_WINDOW_WORKSPACE,
} from "../../platform/workspace/common/workspace.js";
import {
  createBrowserWorkbenchContextMenuService,
} from "../services/contextmenu/browser/contextMenuService.js";
import {
  createBrowserTitlebarPart,
} from "./parts/titlebar/titlebarPart.js";
import { startWorkbench } from "./workbench.js";

/** Starts a browser-hosted product workbench with the shared web adapters. */
export function startBrowserWorkbench(
  product: ProductConfiguration,
  api: ZetaRendererApi,
  container: HTMLElement | null,
): IDisposable {
  installBaseUiStyles();
  return startWorkbench({
    product,
    api,
    container,
    workspace: UNKNOWN_EMPTY_WINDOW_WORKSPACE,
    createContextMenuService: createBrowserWorkbenchContextMenuService,
    createTitlebarPart: createBrowserTitlebarPart,
  });
}
