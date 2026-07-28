import { createServiceIdentifier, } from "../../instantiation/common/instantiation.js";
/** Visual severity used by a modal message dialog. */
export var DialogSeverity;
(function (DialogSeverity) {
    DialogSeverity["Info"] = "info";
    DialogSeverity["Warning"] = "warning";
    DialogSeverity["Error"] = "error";
})(DialogSeverity || (DialogSeverity = {}));
/** Result returned by a host-specific dialog handler. */
export var DialogResult;
(function (DialogResult) {
    DialogResult["Primary"] = "primary";
    DialogResult["Cancel"] = "cancel";
})(DialogResult || (DialogResult = {}));
export const IDialogService = createServiceIdentifier("dialogService");
