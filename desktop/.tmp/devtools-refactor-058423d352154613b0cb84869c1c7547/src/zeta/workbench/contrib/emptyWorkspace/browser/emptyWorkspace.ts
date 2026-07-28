import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import type {
  ICommandService,
} from "../../../../platform/commands/common/commands.js";
import {
  type IWorkspaceContextService,
  WorkbenchState,
} from "../../../../platform/workspace/common/workspace.js";
import type {
  IWorkbenchLayoutService,
} from "../../../browser/layout.js";
import type {
  IEditorPart,
} from "../../../browser/parts/editor/editorPart.js";
import { StartTurnCommandId } from "../../turn/common/turnCommands.js";
import { EmptyView } from "./emptyView.js";

/** Projects an empty workspace into the editor and Workbench layout. */
export class EmptyWorkspaceContribution extends DisposableOwner {
  constructor(
    workspaceContextService: IWorkspaceContextService,
    editorPart: IEditorPart,
    layoutService: IWorkbenchLayoutService,
    commandService: ICommandService,
  ) {
    super();
    if (
      workspaceContextService.getWorkbenchState() !== WorkbenchState.EMPTY
    ) {
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
