import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import { IDialogsModel, IWorkbenchDialogHandler, } from "../../../common/dialogs.js";
import { registerWorkbenchContribution, WorkbenchPhase, } from "../../../common/contributions.js";
/**
 * Serially presents the queue owned by the workbench dialog service.
 */
export class DialogHandlerContribution extends DisposableOwner {
    #model;
    #handler;
    #active;
    #disposed = false;
    constructor(model, handler) {
        super();
        this.#model = model;
        this.#handler = handler;
        this.defer(() => {
            this.#disposed = true;
            const active = this.#active;
            this.#active = undefined;
            active?.controller.abort();
            active?.item.cancel();
        });
        this.own(model.onDidCloseDialog(({ item }) => {
            if (this.#active?.item === item) {
                this.#active.controller.abort();
            }
        }));
        this.own(model.onWillShowDialog(() => this.#processDialogs()));
        this.#processDialogs();
    }
    #processDialogs() {
        if (this.#disposed || this.#active)
            return;
        const item = this.#model.dialogs[0];
        if (!item)
            return;
        const active = {
            item,
            controller: new AbortController(),
        };
        this.#active = active;
        void this.#show(active);
    }
    async #show(active) {
        let outcome;
        try {
            outcome = {
                kind: "result",
                result: await this.#handler.showDialog(active.item.request, active.controller.signal),
            };
        }
        catch (handlerError) {
            outcome = { kind: "error", error: handlerError };
        }
        if (this.#active !== active)
            return;
        this.#active = undefined;
        try {
            if (outcome.kind === "result") {
                active.item.close(outcome.result);
            }
            else {
                active.item.fail(outcome.error);
            }
        }
        finally {
            this.#processDialogs();
        }
    }
}
registerWorkbenchContribution("workbench.contrib.dialogHandler", WorkbenchPhase.BlockStartup, (accessor) => new DialogHandlerContribution(accessor.get(IDialogsModel), accessor.get(IWorkbenchDialogHandler)));
