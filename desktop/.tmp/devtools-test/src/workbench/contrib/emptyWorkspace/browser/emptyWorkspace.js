import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import { StartTurnCommandId } from "../../turn/common/turnCommands.js";
import { EmptyView } from "./emptyView.js";
/** Projects an empty workspace into the editor and Workbench layout. */
export class EmptyWorkspaceContribution extends DisposableOwner {
    constructor(workspaceContextService, editorPart, layoutService, commandService) {
        super();
        if (workspaceContextService.getWorkbenchState() !== 1 /* WorkbenchState.EMPTY */) {
            return;
        }
        const view = this.own(new EmptyView({
            ownerDocument: editorPart.element.ownerDocument,
            startTurn: async () => {
                await commandService.executeCommand(StartTurnCommandId);
            },
        }));
        editorPart.setContent(view.element);
        layoutService.hideParts(["sidebar", "auxiliarybar"]);
    }
}
