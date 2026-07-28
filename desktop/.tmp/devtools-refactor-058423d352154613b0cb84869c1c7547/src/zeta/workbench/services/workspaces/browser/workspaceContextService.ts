import type { URI } from "../../../../base/common/uri.js";
import type {
  IAnyWorkspaceIdentifier,
  IWorkspace,
  IWorkspaceContextService,
  IWorkspaceFolder,
} from "../../../../platform/workspace/common/workspace.js";
import {
  isSingleFolderWorkspaceIdentifier,
  isWorkspaceIdentifier,
  WorkbenchState,
} from "../../../../platform/workspace/common/workspace.js";

/** Immutable renderer projection of the workspace hosted by this window. */
export class WorkspaceContextService implements IWorkspaceContextService {
  readonly #workspace: IWorkspace;

  constructor(workspace: IAnyWorkspaceIdentifier) {
    this.#workspace = resolveWorkspace(workspace);
  }

  getWorkspace(): IWorkspace {
    return this.#workspace;
  }

  getWorkbenchState(): WorkbenchState {
    if (this.#workspace.configuration) {
      return WorkbenchState.WORKSPACE;
    }
    if (this.#workspace.folders.length === 1) {
      return WorkbenchState.FOLDER;
    }
    return WorkbenchState.EMPTY;
  }
}

function resolveWorkspace(
  identifier: IAnyWorkspaceIdentifier,
): IWorkspace {
  if (isWorkspaceIdentifier(identifier)) {
    return Object.freeze({
      id: identifier.id,
      folders: Object.freeze([]),
      configuration: identifier.configPath,
      name: workspaceName(identifier.configPath),
    });
  }
  if (isSingleFolderWorkspaceIdentifier(identifier)) {
    const folder: IWorkspaceFolder = Object.freeze({
      uri: identifier.uri,
      name: resourceName(identifier.uri),
      index: 0,
    });
    return Object.freeze({
      id: identifier.id,
      folders: Object.freeze([folder]),
    });
  }
  return Object.freeze({
    id: identifier.id,
    folders: Object.freeze([]),
  });
}

function workspaceName(configPath: URI): string {
  const name = resourceName(configPath);
  const extension = ".zeta-workspace";
  return name.toLowerCase().endsWith(extension)
    ? name.slice(0, -extension.length) || name
    : name;
}

function resourceName(resource: URI): string {
  const path = decodeURIComponent(resource.path).replace(/\/+$/, "");
  const name = path.slice(path.lastIndexOf("/") + 1);
  return name || resource.authority || resource.toString();
}
