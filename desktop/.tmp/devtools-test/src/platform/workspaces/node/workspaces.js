import { createHash } from "node:crypto";
import { realpath, stat } from "node:fs/promises";
import { extname, resolve } from "node:path";
import { URI } from "../../../base/common/uri.js";
import { UNKNOWN_EMPTY_WINDOW_WORKSPACE, } from "../../workspace/common/workspace.js";
const ZETA_WORKSPACE_EXTENSION = ".zeta-workspace";
/** Native path service used by the Electron main process. */
export const nodeWorkspacePathService = {
    async resolvePath(path) {
        const canonicalPath = await realpath(path);
        const metadata = await stat(canonicalPath);
        const kind = metadata.isDirectory()
            ? 0 /* WorkspacePathKind.Directory */
            : metadata.isFile()
                ? 1 /* WorkspacePathKind.File */
                : 2 /* WorkspacePathKind.Other */;
        return { kind, path: canonicalPath };
    },
};
/**
 * Resolves one launch target into a stable folder or workspace identity.
 *
 * A loose file is not a workspace, so it resolves to an empty-window identity.
 */
export async function resolveWorkspaceOpenTarget(target, cwd, pathService = nodeWorkspacePathService) {
    const requestedPath = resolve(cwd, target.path);
    const resolved = await pathService.resolvePath(requestedPath);
    if (target.kind === 1 /* WorkspaceOpenTargetKind.Folder */ &&
        resolved.kind !== 0 /* WorkspacePathKind.Directory */) {
        throw new Error(`Workspace folder is not a directory: ${target.path}`);
    }
    if (target.kind === 2 /* WorkspaceOpenTargetKind.Workspace */ &&
        resolved.kind !== 1 /* WorkspacePathKind.File */) {
        throw new Error(`Workspace configuration is not a file: ${target.path}`);
    }
    if (resolved.kind === 0 /* WorkspacePathKind.Directory */) {
        return getSingleFolderWorkspaceIdentifier(URI.file(resolved.path));
    }
    if (resolved.kind === 1 /* WorkspacePathKind.File */ &&
        (target.kind === 2 /* WorkspaceOpenTargetKind.Workspace */ ||
            extname(resolved.path).toLowerCase() === ZETA_WORKSPACE_EXTENSION)) {
        return getWorkspaceIdentifier(URI.file(resolved.path));
    }
    return UNKNOWN_EMPTY_WINDOW_WORKSPACE;
}
/** Creates the stable identity of one multi-root workspace file. */
export function getWorkspaceIdentifier(configPath) {
    return Object.freeze({
        id: stableWorkspaceId(configPath),
        configPath,
    });
}
/** Creates the stable identity of one single-folder workspace. */
export function getSingleFolderWorkspaceIdentifier(uri) {
    return Object.freeze({
        id: stableWorkspaceId(uri),
        uri,
    });
}
function stableWorkspaceId(uri) {
    const identity = process.platform === "linux"
        ? uri.toString()
        : uri.toString().toLowerCase();
    return createHash("sha256").update(identity).digest("hex");
}
