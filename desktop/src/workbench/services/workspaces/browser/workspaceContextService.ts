import type {
  IWorkspaceContext,
  IWorkspaceContextService,
  WorkbenchState,
} from "../../../../platform/workspace/common/workspace.js";

/** Immutable renderer projection of the project hosted by this window. */
export class WorkspaceContextService implements IWorkspaceContextService {
  readonly #workspace: IWorkspaceContext;

  constructor(workspace: IWorkspaceContext) {
    this.#workspace = workspace;
  }

  getWorkspace(): IWorkspaceContext {
    return this.#workspace;
  }

  getWorkbenchState(): WorkbenchState {
    return this.#workspace.state;
  }
}
