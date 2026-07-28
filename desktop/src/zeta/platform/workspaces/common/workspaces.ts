/** How a user-facing launch path should be interpreted. */
export const enum WorkspaceOpenTargetKind {
  Automatic,
  Folder,
  Workspace,
}

/** One folder or workspace-file target requested by a window launch. */
export interface IWorkspaceOpenTarget {
  readonly kind: WorkspaceOpenTargetKind;
  readonly path: string;
}
