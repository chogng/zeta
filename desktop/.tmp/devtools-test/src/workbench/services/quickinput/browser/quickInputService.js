import { addDisposableListener, isHTMLElement, stopEvent, } from "../../../../base/browser/dom.js";
import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import { BrowserQuickPick, } from "../../../../platform/quickinput/browser/quickPick.js";
import { RawContextKey } from "../../../../platform/contextkey/common/contextkey.js";
export const InQuickInputContext = new RawContextKey("inQuickInput", false);
/** Window-scoped host shared by every short-lived Quick Input controller. */
export class WorkbenchQuickInputService extends DisposableOwner {
    #host;
    #ownerDocument;
    #inQuickInput;
    #quickPicks = new Set();
    #active;
    #focusToRestore;
    constructor(options) {
        super();
        this.#ownerDocument = options.container.ownerDocument;
        this.#inQuickInput =
            InQuickInputContext.bindTo(options.contextKeyService);
        this.#host = this.#ownerDocument.createElement("div");
        this.#host.className = "zeta-quick-input-host";
        this.#host.hidden = true;
        options.container.append(this.#host);
        this.own(addDisposableListener(this.#host, "mousedown", (event) => {
            if (event.target !== this.#host)
                return;
            stopEvent(event);
            this.#active?.hide();
        }));
        this.defer(() => {
            for (const quickPick of [...this.#quickPicks]) {
                quickPick.dispose();
            }
            this.#quickPicks.clear();
            this.#active = undefined;
            this.#focusToRestore = undefined;
            this.#inQuickInput.reset();
            this.#host.remove();
        });
    }
    createQuickPick() {
        let quickPick;
        quickPick = new BrowserQuickPick({
            ownerDocument: this.#ownerDocument,
            onShow: (candidate) => this.#show(candidate),
            onHide: (candidate) => this.#hide(candidate),
            onDispose: (candidate) => {
                this.#quickPicks.delete(candidate);
                this.#hide(candidate);
            },
        });
        this.#quickPicks.add(quickPick);
        return quickPick;
    }
    #show(quickPick) {
        if (this.#active === quickPick) {
            quickPick.focus();
            return;
        }
        this.#active?.hide();
        const focused = this.#ownerDocument.activeElement;
        this.#focusToRestore = isHTMLElement(focused)
            ? focused
            : undefined;
        this.#active = quickPick;
        this.#host.replaceChildren(quickPick.element);
        this.#host.hidden = false;
        this.#inQuickInput.set(true);
        quickPick.focus();
    }
    #hide(quickPick) {
        if (this.#active !== quickPick)
            return;
        this.#active = undefined;
        this.#host.replaceChildren();
        this.#host.hidden = true;
        this.#inQuickInput.reset();
        const focusToRestore = this.#focusToRestore;
        this.#focusToRestore = undefined;
        if (focusToRestore?.isConnected)
            focusToRestore.focus();
    }
}
