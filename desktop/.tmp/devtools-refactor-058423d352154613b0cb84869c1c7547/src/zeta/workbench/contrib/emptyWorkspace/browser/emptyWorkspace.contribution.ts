import {
  ICommandService,
} from "../../../../platform/commands/common/commands.js";
import {
  IWorkspaceContextService as WorkspaceContextServiceId,
} from "../../../../platform/workspace/common/workspace.js";
import {
  registerWorkbenchContribution,
  WorkbenchPhase,
} from "../../../common/contributions.js";
import {
  IWorkbenchLayoutService,
} from "../../../browser/layout.js";
import { IEditorPart } from "../../../browser/parts/editor/editorPart.js";
import { EmptyWorkspaceContribution } from "./emptyWorkspace.js";
import "./media/emptyView.css";

registerWorkbenchContribution(
  "workbench.contrib.emptyWorkspace",
  WorkbenchPhase.BlockRestore,
  (accessor) => new EmptyWorkspaceContribution(
    accessor.get(WorkspaceContextServiceId),
    accessor.get(IEditorPart),
    accessor.get(IWorkbenchLayoutService),
    accessor.get(ICommandService),
  ),
);
