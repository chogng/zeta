import { URI } from "../../../base/common/uri.js";
import { createServiceIdentifier, } from "../../instantiation/common/instantiation.js";
export const IWorkspaceContextService = createServiceIdentifier("workspaceContextService");
/** Fallback identity for a window whose durable empty-workspace ID is unknown. */
export const UNKNOWN_EMPTY_WINDOW_WORKSPACE = Object.freeze({ id: "empty-window" });
/** Returns whether a value identifies one folder. */
export function isSingleFolderWorkspaceIdentifier(value) {
    const candidate = value;
    return isNonEmptyString(candidate?.id) && candidate?.uri instanceof URI;
}
/** Returns whether a value identifies a multi-root workspace file. */
export function isWorkspaceIdentifier(value) {
    const candidate = value;
    return isNonEmptyString(candidate?.id) &&
        candidate?.configPath instanceof URI;
}
/** Returns whether a value identifies an empty workbench. */
export function isEmptyWorkspaceIdentifier(value) {
    const candidate = value;
    return isNonEmptyString(candidate?.id) &&
        !isSingleFolderWorkspaceIdentifier(value) &&
        !isWorkspaceIdentifier(value);
}
/** Derives workbench state without storing a second workspace discriminator. */
export function workbenchStateFromWorkspaceIdentifier(workspace) {
    if (isWorkspaceIdentifier(workspace)) {
        return 3 /* WorkbenchState.WORKSPACE */;
    }
    if (isSingleFolderWorkspaceIdentifier(workspace)) {
        return 2 /* WorkbenchState.FOLDER */;
    }
    return 1 /* WorkbenchState.EMPTY */;
}
/** Converts a workspace identity into an IPC-safe plain object. */
export function serializeWorkspaceIdentifier(workspace) {
    if (isWorkspaceIdentifier(workspace)) {
        return {
            id: workspace.id,
            configPath: workspace.configPath.toString(),
        };
    }
    if (isSingleFolderWorkspaceIdentifier(workspace)) {
        return {
            id: workspace.id,
            uri: workspace.uri.toString(),
        };
    }
    return { id: workspace.id };
}
/** Validates and revives a workspace identity received over IPC. */
export function parseWorkspaceIdentifier(value) {
    const record = exactRecord(value);
    const id = nonEmptyString(record.id, "workspace id");
    if ("configPath" in record) {
        requireExactKeys(record, ["configPath", "id"]);
        return Object.freeze({
            id,
            configPath: fileUri(record.configPath, "workspace config path"),
        });
    }
    if ("uri" in record) {
        requireExactKeys(record, ["id", "uri"]);
        return Object.freeze({
            id,
            uri: fileUri(record.uri, "workspace folder uri"),
        });
    }
    requireExactKeys(record, ["id"]);
    return id === UNKNOWN_EMPTY_WINDOW_WORKSPACE.id
        ? UNKNOWN_EMPTY_WINDOW_WORKSPACE
        : Object.freeze({ id });
}
function exactRecord(value) {
    if (typeof value !== "object" || value === null || Array.isArray(value)) {
        throw new Error("workspace identifier must be an object");
    }
    return value;
}
function requireExactKeys(value, expected) {
    const actual = Object.keys(value).sort();
    if (actual.length !== expected.length ||
        actual.some((key, index) => key !== expected[index])) {
        throw new Error(`workspace identifier must contain exactly: ${expected.join(", ")}`);
    }
}
function fileUri(value, field) {
    if (typeof value !== "string") {
        throw new Error(`${field} must be a string`);
    }
    const uri = URI.parse(value);
    if (uri.scheme !== "file") {
        throw new Error(`${field} must use the file scheme`);
    }
    if (uri.query || uri.fragment) {
        throw new Error(`${field} must not contain a query or fragment`);
    }
    return uri;
}
function nonEmptyString(value, field) {
    if (!isNonEmptyString(value)) {
        throw new Error(`${field} must be a non-empty string`);
    }
    return value;
}
function isNonEmptyString(value) {
    return typeof value === "string" && value.trim().length > 0;
}
