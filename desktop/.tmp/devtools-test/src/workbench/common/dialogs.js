import { Emitter } from "../../base/common/event.js";
import { DisposableOwner } from "../../base/common/lifecycle.js";
import { DialogResult, } from "../../platform/dialogs/common/dialogs.js";
import { createServiceIdentifier, } from "../../platform/instantiation/common/instantiation.js";
/** Window-scoped dialog model owned by the workbench dialog service. */
export const IDialogsModel = createServiceIdentifier("dialogsModel");
/** Host-specific dialog renderer consumed by the dialog contribution. */
export const IWorkbenchDialogHandler = createServiceIdentifier("workbenchDialogHandler");
/**
 * Owns pending workbench dialogs without depending on browser presentation.
 */
export class DialogsModel extends DisposableOwner {
    #onWillShowDialog = this.own(new Emitter());
    #onDidCloseDialog = this.own(new Emitter());
    #dialogs = [];
    #disposed = false;
    onWillShowDialog = this.#onWillShowDialog.event;
    onDidCloseDialog = this.#onDidCloseDialog.event;
    constructor() {
        super();
        this.defer(() => {
            this.#disposed = true;
            for (const item of [...this.#dialogs])
                item.cancel();
        });
    }
    get dialogs() {
        return [...this.#dialogs];
    }
    show(request) {
        if (this.#disposed) {
            throw new ReferenceError("DialogsModel is already disposed");
        }
        let resolveResult;
        let rejectResult;
        const result = new Promise((resolve, reject) => {
            resolveResult = resolve;
            rejectResult = reject;
        });
        let settled = false;
        const item = {
            request,
            close: (dialogResult) => {
                if (settled)
                    return;
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
                if (settled)
                    return;
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
    #remove(item) {
        const index = this.#dialogs.indexOf(item);
        if (index >= 0)
            this.#dialogs.splice(index, 1);
    }
}
