import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import { DialogResult, } from "../../../../platform/dialogs/common/dialogs.js";
import { DialogsModel } from "../../../common/dialogs.js";
/**
 * Maps the platform dialog API onto the workbench-owned dialog model.
 */
export class DialogService extends DisposableOwner {
    model = this.own(new DialogsModel());
    async showMessage(options) {
        const handle = this.model.show({
            kind: "message",
            ...options,
        });
        await handle.result;
    }
    async confirm(options) {
        const handle = this.model.show({
            kind: "confirmation",
            ...options,
        });
        return await handle.result === DialogResult.Primary;
    }
}
