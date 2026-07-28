import { Emitter, type Event } from "../../base/common/event.js";
import { DisposableOwner } from "../../base/common/lifecycle.js";
import {
  type DialogRequest,
  DialogResult,
  type IDialogHandler,
} from "../../platform/dialogs/common/dialogs.js";
import {
  createServiceIdentifier,
} from "../../platform/instantiation/common/instantiation.js";

/** One dialog exposed to a workbench dialog renderer. */
export interface IDialogViewItem {
  readonly request: DialogRequest;

  close(result: DialogResult): void;
  cancel(): void;
  fail(error: unknown): void;
}

/** Handle returned to the caller that enqueued a dialog. */
export interface IDialogHandle {
  readonly item: IDialogViewItem;
  readonly result: Promise<DialogResult>;
}

/** Change emitted after one dialog leaves the model. */
export type IDialogCloseEvent =
  | {
    readonly kind: "result";
    readonly item: IDialogViewItem;
    readonly result: DialogResult;
  }
  | {
    readonly kind: "error";
    readonly item: IDialogViewItem;
    readonly error: unknown;
  };

/**
 * Window-scoped dialog queue shared between dialog services and renderers.
 */
export interface IDialogsModel {
  readonly onWillShowDialog: Event<IDialogViewItem>;
  readonly onDidCloseDialog: Event<IDialogCloseEvent>;
  readonly dialogs: readonly IDialogViewItem[];

  show(request: DialogRequest): IDialogHandle;
}

/** Window-scoped dialog model owned by the workbench dialog service. */
export const IDialogsModel =
  createServiceIdentifier<IDialogsModel>("dialogsModel");

/** Host-specific dialog renderer consumed by the dialog contribution. */
export const IWorkbenchDialogHandler =
  createServiceIdentifier<IDialogHandler>("workbenchDialogHandler");

/**
 * Owns pending workbench dialogs without depending on browser presentation.
 */
export class DialogsModel
  extends DisposableOwner
  implements IDialogsModel {
  readonly #onWillShowDialog =
    this.own(new Emitter<IDialogViewItem>());
  readonly #onDidCloseDialog =
    this.own(new Emitter<IDialogCloseEvent>());
  readonly #dialogs: IDialogViewItem[] = [];
  #disposed = false;

  readonly onWillShowDialog = this.#onWillShowDialog.event;
  readonly onDidCloseDialog = this.#onDidCloseDialog.event;

  constructor() {
    super();
    this.defer(() => {
      this.#disposed = true;
      for (const item of [...this.#dialogs]) item.cancel();
    });
  }

  get dialogs(): readonly IDialogViewItem[] {
    return [...this.#dialogs];
  }

  show(request: DialogRequest): IDialogHandle {
    if (this.#disposed) {
      throw new ReferenceError("DialogsModel is already disposed");
    }

    let resolveResult!: (result: DialogResult) => void;
    let rejectResult!: (error: unknown) => void;
    const result = new Promise<DialogResult>((resolve, reject) => {
      resolveResult = resolve;
      rejectResult = reject;
    });
    let settled = false;

    const item: IDialogViewItem = {
      request,
      close: (dialogResult) => {
        if (settled) return;
        settled = true;
        this.#remove(item);
        resolveResult(dialogResult);
        this.#onDidCloseDialog.fire({
          kind: "result",
          item,
          result: dialogResult,
        });
      },
      cancel: () => item.close(DialogResult.Cancel),
      fail: (error) => {
        if (settled) return;
        settled = true;
        this.#remove(item);
        rejectResult(error);
        this.#onDidCloseDialog.fire({ kind: "error", item, error });
      },
    };
    this.#dialogs.push(item);
    this.#onWillShowDialog.fire(item);
    return { item, result };
  }

  #remove(item: IDialogViewItem): void {
    const index = this.#dialogs.indexOf(item);
    if (index >= 0) this.#dialogs.splice(index, 1);
  }
}
