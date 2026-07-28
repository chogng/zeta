import { cloneDocumentStyles } from "../../base/browser/domStylesheets.js";
import {
  isRegisteredWindow,
  mainWindow,
  registerWindow,
} from "../../base/browser/window.js";
import { DisposableOwner } from "../../base/common/lifecycle.js";
import { environment } from "../../base/common/platform.js";
import type { ProductId } from "../../product/common/product.js";
import type {
  WorkbenchState,
} from "../../platform/workspace/common/workspace.js";
import {
  workbenchStateToContextValue,
} from "../common/contextkeys.js";

export interface WorkbenchWindowOptions {
  readonly root: HTMLElement;
  readonly productId: ProductId;
  readonly workbenchState: WorkbenchState;
}

/**
 * Owns the browser-window identity and document integration for one Workbench.
 *
 * Workbench services and Parts use `ownerDocument`; this class owns the
 * corresponding window registration, stylesheet projection, root attributes,
 * and deterministic teardown.
 */
export class WorkbenchWindow extends DisposableOwner {
  readonly root: HTMLElement;
  readonly ownerDocument: Document;
  readonly targetWindow: Window | null;

  constructor(options: WorkbenchWindowOptions) {
    super();
    this.root = options.root;
    this.ownerDocument = options.root.ownerDocument;
    this.targetWindow = this.ownerDocument.defaultView;

    options.root.classList.add("zeta-workbench");
    options.root.setAttribute("data-product", options.productId);
    options.root.setAttribute("data-runtime", environment.runtime);
    options.root.setAttribute("data-os", environment.os);
    options.root.setAttribute(
      "data-workbench-state",
      workbenchStateToContextValue(options.workbenchState),
    );
    this.defer(() => {
      options.root.classList.remove("zeta-workbench");
      options.root.removeAttribute("data-product");
      options.root.removeAttribute("data-runtime");
      options.root.removeAttribute("data-os");
      options.root.removeAttribute("data-workbench-state");
      options.root.replaceChildren();
    });

    if (
      this.targetWindow &&
      !isRegisteredWindow(this.targetWindow)
    ) {
      this.own(registerWindow(this.targetWindow));
    }
    if (this.ownerDocument !== mainWindow.document) {
      this.own(cloneDocumentStyles(
        mainWindow.document,
        this.ownerDocument,
      ));
    }
  }
}
