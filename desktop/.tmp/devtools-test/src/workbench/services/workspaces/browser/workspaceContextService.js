import { isSingleFolderWorkspaceIdentifier, isWorkspaceIdentifier, } from "../../../../platform/workspace/common/workspace.js";
/** Immutable renderer projection of the workspace hosted by this window. */
export class WorkspaceContextService {
    #workspace;
    constructor(workspace) {
        this.#workspace = resolveWorkspace(workspace);
    }
    getWorkspace() {
        return this.#workspace;
    }
    getWorkbenchState() {
        if (this.#workspace.configuration) {
            return 3 /* WorkbenchState.WORKSPACE */;
        }
        if (this.#workspace.folders.length === 1) {
            return 2 /* WorkbenchState.FOLDER */;
        }
        return 1 /* WorkbenchState.EMPTY */;
    }
}
function resolveWorkspace(identifier) {
    if (isWorkspaceIdentifier(identifier)) {
        return Object.freeze({
            id: identifier.id,
            folders: Object.freeze([]),
            configuration: identifier.configPath,
            name: workspaceName(identifier.configPath),
        });
    }
    if (isSingleFolderWorkspaceIdentifier(identifier)) {
        const folder = Object.freeze({
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
function workspaceName(configPath) {
    const name = resourceName(configPath);
    const extension = ".zeta-workspace";
    return name.toLowerCase().endsWith(extension)
        ? name.slice(0, -extension.length) || name
        : name;
}
function resourceName(resource) {
    const path = decodeURIComponent(resource.path).replace(/\/+$/, "");
    const name = path.slice(path.lastIndexOf("/") + 1);
    return name || resource.authority || resource.toString();
}
