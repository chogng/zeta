import { DisposableOwner } from "../../../base/common/lifecycle.js";
import {
  createServiceIdentifier,
} from "../../instantiation/common/instantiation.js";

/** Visual severity used by a modal message dialog. */
export enum DialogSeverity {
  Info = "info",
  Warning = "warning",
  Error = "error",
}

/** Content shared by message and confirmation dialogs. */
export interface IDialogOptions {
  readonly title?: string;
  readonly message: string;
  readonly detail?: string;
}

/** Options for a modal message that has one dismiss button. */
export interface IMessageDialogOptions extends IDialogOptions {
  readonly severity: DialogSeverity;
  readonly primaryButton?: string;
}

/** Options for a modal question with explicit confirm and cancel actions. */
export interface IConfirmationDialogOptions extends IDialogOptions {
  readonly primaryButton?: string;
  readonly cancelButton?: string;
}

/** Requests understood by a host-specific dialog handler. */
export type DialogRequest =
  | ({
    readonly kind: "message";
  } & IMessageDialogOptions)
  | ({
    readonly kind: "confirmation";
  } & IConfirmationDialogOptions);

/** Result returned by a host-specific dialog handler. */
export enum DialogResult {
  Primary = "primary",
  Cancel = "cancel",
}

/**
 * Presents one dialog using the active host UI.
 *
 * Implementations must observe `signal` and settle as cancelled after abort.
 */
export interface IDialogHandler {
  showDialog(
    request: DialogRequest,
    signal: AbortSignal,
  ): Promise<DialogResult>;
}

/** Window-scoped access to modal workbench dialogs. */
export interface IDialogService {
  showMessage(options: IMessageDialogOptions): Promise<void>;
  confirm(options: IConfirmationDialogOptions): Promise<boolean>;
}

export const IDialogService =
  createServiceIdentifier<IDialogService>("dialogService");

interface IPendingDialog {
  readonly request: DialogRequest;
  readonly resolve: (result: DialogResult) => void;
  readonly reject: (error: unknown) => void;
}

interface IActiveDialog {
  readonly pending: IPendingDialog;
  readonly controller: AbortController;
}

/**
 * Serializes modal requests and delegates their presentation to one handler.
 */
export class DialogService extends DisposableOwner
  implements IDialogService {
  readonly #handler: IDialogHandler;
  readonly #queue: IPendingDialog[] = [];
  #active: IActiveDialog | undefined;
  #disposed = false;

  constructor(handler: IDialogHandler) {
    super();
    this.#handler = handler;
    this.defer(() => {
      this.#disposed = true;
      const active = this.#active;
      this.#active = undefined;
      if (active) {
        active.controller.abort();
        active.pending.resolve(DialogResult.Cancel);
      }
      for (const pending of this.#queue.splice(0)) {
        pending.resolve(DialogResult.Cancel);
      }
    });
  }

  async showMessage(options: IMessageDialogOptions): Promise<void> {
    await this.#enqueue({
      kind: "message",
      ...options,
    });
  }

  async confirm(options: IConfirmationDialogOptions): Promise<boolean> {
    const result = await this.#enqueue({
      kind: "confirmation",
      ...options,
    });
    return result === DialogResult.Primary;
  }

  #enqueue(request: DialogRequest): Promise<DialogResult> {
    if (this.#disposed) {
      return Promise.reject(
        new ReferenceError("DialogService is already disposed"),
      );
    }

    const result = new Promise<DialogResult>((resolve, reject) => {
      this.#queue.push({
        request,
        resolve,
        reject,
      });
    });
    this.#showNext();
    return result;
  }

  #showNext(): void {
    if (this.#disposed || this.#active) return;
    const pending = this.#queue.shift();
    if (!pending) return;

    const active: IActiveDialog = {
      pending,
      controller: new AbortController(),
    };
    this.#active = active;
    void this.#run(active);
  }

  async #run(active: IActiveDialog): Promise<void> {
    try {
      const result = await this.#handler.showDialog(
        active.pending.request,
        active.controller.signal,
      );
      this.#complete(active, result);
    } catch (error) {
      this.#fail(active, error);
    }
  }

  #complete(active: IActiveDialog, result: DialogResult): void {
    if (this.#active !== active) return;
    this.#active = undefined;
    active.pending.resolve(result);
    this.#showNext();
  }

  #fail(active: IActiveDialog, error: unknown): void {
    if (this.#active !== active) return;
    this.#active = undefined;
    active.pending.reject(error);
    this.#showNext();
  }
}
