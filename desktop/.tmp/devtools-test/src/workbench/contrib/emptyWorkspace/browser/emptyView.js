import { Button } from "../../../../base/browser/ui/button/button.js";
import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import { LxIcon } from "../../../../base/common/lxicons.js";
/** Central call to action shown when the window has no folder or workspace. */
export class EmptyView extends DisposableOwner {
    element;
    constructor({ ownerDocument, startTurn, }) {
        super();
        const element = ownerDocument.createElement("div");
        this.element = element;
        this.defer(() => element.remove());
        element.className = "zeta-empty-workspace-view";
        const title = ownerDocument.createElement("h1");
        title.textContent = "No folder open";
        const description = ownerDocument.createElement("p");
        description.textContent =
            "Start a conversation without project context.";
        const button = this.own(new Button({
            label: "Start conversation",
            ownerDocument,
            icon: LxIcon.start,
            onClick: () => {
                void startTurn();
            },
        }));
        element.append(title, description, button.element);
    }
}
