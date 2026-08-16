import { cloneDocumentStyles } from "../../base/browser/domStylesheets.js";
import {
  isRegisteredWindow,
  mainWindow,
  registerWindow,
} from "../../base/browser/window.js";
import { Emitter } from "../../base/common/event.js";
import { DisposableOwner } from "../../base/common/lifecycle.js";
import { environment } from "../../base/common/platform.js";
import type { ProductId } from "../../product/common/product.js";
import type {
  WorkbenchState,
} from "../../platform/workspace/common/workspace.js";
import {
  workbenchStateToContextValue,
} from "../common/contextkeys.js";
import { type IWorkbenchHostService, type WorkbenchTextDownload } from "../services/host/common/workbenchHostService.js";

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
export class WorkbenchWindow
  extends DisposableOwner
  implements IWorkbenchHostService {
  readonly root: HTMLElement;
  readonly ownerDocument: Document;
  readonly targetWindow: Window | null;
  private readonly errorEmitter = this.own(new Emitter<{ readonly kind: "runtimeError" | "unhandledRejection"; readonly message: string; readonly source: string | undefined }>());
  readonly onDidError = this.errorEmitter.event;

  constructor(options: WorkbenchWindowOptions) {
    super();
    this.root = options.root;
    this.ownerDocument = options.root.ownerDocument;
    this.targetWindow = this.ownerDocument.defaultView;

    options.root.classList.add("zeta-workbench");
    options.root.setAttribute("data-product", options.productId);
    options.root.setAttribute("data-runtime", environment.runtime);
    options.root.setAttribute("data-os", environment.os);
    this.setWorkbenchState(options.workbenchState);
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
    if (this.targetWindow) {
      const onError = (event: ErrorEvent): void => this.errorEmitter.fire({ kind: "runtimeError", message: event.message || errorMessage(event.error), source: event.filename ? `${event.filename}:${event.lineno}:${event.colno}` : undefined });
      const onUnhandledRejection = (event: PromiseRejectionEvent): void => this.errorEmitter.fire({ kind: "unhandledRejection", message: errorMessage(event.reason), source: undefined });
      this.targetWindow.addEventListener("error", onError);
      this.targetWindow.addEventListener("unhandledrejection", onUnhandledRejection);
      this.defer(() => {
        this.targetWindow?.removeEventListener("error", onError);
        this.targetWindow?.removeEventListener("unhandledrejection", onUnhandledRejection);
      });
    }
  }

  downloadText(download: WorkbenchTextDownload): void {
    if (!this.targetWindow) throw new Error("Text download requires a browser window");
    const targetWindow = this.targetWindow as Window & typeof globalThis;
    const url = targetWindow.URL.createObjectURL(new targetWindow.Blob([download.content], { type: download.mediaType }));
    const anchor = this.ownerDocument.createElement("a");
    anchor.href = url;
    anchor.download = download.fileName;
    anchor.click();
    targetWindow.setTimeout(() => targetWindow.URL.revokeObjectURL(url), 0);
  }

  setWorkbenchState(state: WorkbenchState): void {
    this.root.setAttribute(
      "data-workbench-state",
      workbenchStateToContextValue(state),
    );
  }
}

function errorMessage(error: unknown): string {
  if (error instanceof Error) return error.stack || error.message;
  return String(error);
}
