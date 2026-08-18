import { Button } from "../../../base/browser/ui/button/button.js";
import { Dialog } from "../../../base/browser/ui/dialog/dialog.js";
import {
  getActiveElement,
  restoreFocus,
} from "../../../base/browser/focus.js";
import { addDisposableListener, isHTMLElement, h } from "../../../base/browser/dom.js";
import { DisposableStore } from "../../../base/common/lifecycle.js";
import {
  type DialogRequest,
  DialogResult,
  DialogSeverity,
  type IDialogHandler,
} from "../common/dialogs.js";

/** Presents workbench dialogs with browser-native modal semantics. */
export class BrowserDialogHandler implements IDialogHandler {
  private readonly container: HTMLElement;

  constructor(container: HTMLElement) {
    this.container = container;
  }

  async showDialog(
    request: DialogRequest,
    signal: AbortSignal,
  ): Promise<DialogResult> {
    if (signal.aborted) return DialogResult.Cancel;

    const disposables = new DisposableStore();
    const ownerDocument = this.container.ownerDocument;
    const previousFocus = getActiveElement(ownerDocument);
    try {
      const content = createDialogContent(ownerDocument, request);
      const dialog = disposables.add(new Dialog(this.container, {
        title: request.title ?? defaultTitle(request),
        content: content.element,
      }));
      dialog.element.dataset.dialogSeverity =
        request.kind === "message" ? request.severity : "question";

      const primaryButton = disposables.add(new Button(content.actions, {
        label: request.primaryButton ??
          (request.kind === "confirmation" ? "Confirm" : "OK"),
        onClick: () => dialog.close(DialogResult.Primary),
      }));
      primaryButton.element.classList.add("zeta-dialog-primary-button");
      content.actions.append(primaryButton.element);

      if (request.kind === "confirmation") {
        const cancelButton = disposables.add(new Button(content.actions, {
          label: request.cancelButton ?? "Cancel",
          onClick: () => dialog.close(DialogResult.Cancel),
        }));
        content.actions.append(cancelButton.element);
      }

      this.container.append(dialog.element);
      disposables.add(addDisposableListener(signal, "abort", () => {
        dialog.close(DialogResult.Cancel);
      }, { once: true }));

      const result = await dialog.show();
      return result === DialogResult.Primary
        ? DialogResult.Primary
        : DialogResult.Cancel;
    } finally {
      disposables.dispose();
      if (isHTMLElement(previousFocus)) restoreFocus(previousFocus);
    }
  }
}

interface IDialogContent {
  readonly element: HTMLDivElement;
  readonly actions: HTMLElement;
}

function createDialogContent(
  ownerDocument: Document,
  request: DialogRequest,
): IDialogContent {
  const element = h(ownerDocument, "div");
  element.className = "zeta-dialog-content";

  const message = h(ownerDocument, "p");
  message.className = "zeta-dialog-message";
  message.textContent = request.message;
  element.append(message);

  if (request.detail) {
    const detail = h(ownerDocument, "p");
    detail.className = "zeta-dialog-detail";
    detail.textContent = request.detail;
    element.append(detail);
  }

  const actions = h(ownerDocument, "footer");
  actions.className = "zeta-dialog-actions";
  element.append(actions);
  return { element, actions };
}

function defaultTitle(request: DialogRequest): string {
  if (request.kind === "confirmation") return "Confirm";
  switch (request.severity) {
    case DialogSeverity.Warning:
      return "Warning";
    case DialogSeverity.Error:
      return "Error";
    default:
      return "Information";
  }
}
