import { toDisposable } from "../../../base/common/lifecycle.js";
import { URI } from "../../../base/common/uri.js";
import { isEmptyWorkspaceIdentifier, isSingleFolderWorkspaceIdentifier, isWorkspaceIdentifier, workbenchStateFromWorkspaceIdentifier, } from "../../workspace/common/workspace.js";
import { defaultWindowState, WindowMode, } from "../../window/electron-main/window.js";
import { validateWindowState, } from "./windows.js";
const WINDOWS_STATE_STORAGE_KEY = "windowsState";
const WINDOWS_STATE_VERSION = 1;
/**
 * Owns the persisted schema and lifecycle for the main Electron window state.
 *
 * Window placement is associated with a concrete workspace, folder, or empty
 * window backup. The last active window remains the fallback for the first
 * window of a new session, matching VS Code's restore order.
 */
export class WindowsStateHandler {
    #stateService;
    #displayService;
    #workspace;
    #backupPath;
    #workbenchState;
    #onError;
    #windowsState;
    #lastNormalBounds;
    constructor({ stateService, displayService, workspace, backupPath, onError = () => undefined, }) {
        this.#stateService = stateService;
        this.#displayService = displayService;
        this.#workspace = workspace;
        this.#backupPath = backupPath;
        this.#workbenchState = workbenchStateFromWorkspaceIdentifier(workspace);
        this.#onError = onError;
        this.#windowsState = parseWindowsState(this.#stateService.getItem(WINDOWS_STATE_STORAGE_KEY));
    }
    /** Restores an exact workspace match, then last active state, then defaults. */
    restoreWindowState() {
        const exactState = this.#windowsState.openedWindows.find((windowState) => matchesWindowIdentity(windowState, this.#workspace, this.#backupPath));
        const candidates = [
            exactState?.uiState,
            this.#windowsState.lastActiveWindow?.uiState,
        ];
        for (const candidate of candidates) {
            if (!candidate) {
                continue;
            }
            const restoredState = validateWindowState(candidate, this.#displayService.getAllDisplays(), this.#workbenchState);
            if (restoredState) {
                this.#lastNormalBounds = toBounds(restoredState);
                return restoredState;
            }
        }
        return defaultWindowState(this.#workbenchState);
    }
    /** Saves immediately on blur and before the BrowserWindow closes. */
    trackWindow(window) {
        const save = () => {
            void this.saveWindowState(window).catch(this.#onError);
        };
        window.on("blur", save);
        window.on("close", save);
        return toDisposable(() => {
            window.removeListener("blur", save);
            window.removeListener("close", save);
        });
    }
    /** Captures normal bounds and flushes the complete window-session state. */
    async saveWindowState(window) {
        const uiState = this.#captureWindowState(window);
        if (!uiState) {
            return;
        }
        const currentWindow = createWindowStateRecord(this.#workspace, this.#backupPath, uiState);
        const windowsState = {
            lastActiveWindow: currentWindow,
            openedWindows: [currentWindow],
        };
        this.#windowsState = windowsState;
        this.#stateService.setItem(WINDOWS_STATE_STORAGE_KEY, serializeWindowsState(windowsState));
        await this.#stateService.flush();
    }
    #captureWindowState(window) {
        const mode = window.isFullScreen()
            ? WindowMode.Fullscreen
            : window.isMaximized()
                ? WindowMode.Maximized
                : WindowMode.Normal;
        const primaryBounds = readBounds(() => mode === WindowMode.Normal
            ? window.getBounds()
            : window.getNormalBounds());
        const bounds = primaryBounds ??
            readBounds(() => window.getBounds()) ??
            this.#lastNormalBounds;
        if (!bounds) {
            return undefined;
        }
        this.#lastNormalBounds = bounds;
        let displayId;
        if (mode === WindowMode.Fullscreen) {
            const currentBounds = readBounds(() => window.getBounds()) ?? bounds;
            displayId = this.#displayService.getDisplayMatching(currentBounds).id;
        }
        return {
            mode,
            ...bounds,
            displayId,
        };
    }
}
function createWindowStateRecord(workspace, backupPath, uiState) {
    if (isWorkspaceIdentifier(workspace)) {
        return { workspace, uiState };
    }
    if (isSingleFolderWorkspaceIdentifier(workspace)) {
        return { folderUri: workspace.uri, uiState };
    }
    return {
        ...(backupPath === undefined ? {} : { backupPath }),
        uiState,
    };
}
function matchesWindowIdentity(state, workspace, backupPath) {
    if (isWorkspaceIdentifier(workspace)) {
        return state.workspace?.id === workspace.id;
    }
    if (isSingleFolderWorkspaceIdentifier(workspace)) {
        return state.folderUri !== undefined &&
            resourceComparisonKey(state.folderUri) ===
                resourceComparisonKey(workspace.uri);
    }
    return isEmptyWorkspaceIdentifier(workspace) &&
        backupPath !== undefined &&
        state.backupPath === backupPath;
}
function resourceComparisonKey(resource) {
    const value = resource.toString();
    return process.platform === "linux" ? value : value.toLowerCase();
}
function serializeWindowsState(state) {
    return {
        version: WINDOWS_STATE_VERSION,
        ...(state.lastActiveWindow === undefined
            ? {}
            : { lastActiveWindow: serializeWindowStateRecord(state.lastActiveWindow) }),
        openedWindows: state.openedWindows.map(serializeWindowStateRecord),
    };
}
function serializeWindowStateRecord(state) {
    return {
        ...(state.workspace === undefined
            ? {}
            : {
                workspaceIdentifier: {
                    id: state.workspace.id,
                    configURIPath: state.workspace.configPath.toString(),
                },
            }),
        ...(state.folderUri === undefined
            ? {}
            : { folder: state.folderUri.toString() }),
        ...(state.backupPath === undefined
            ? {}
            : { backupPath: state.backupPath }),
        uiState: serializeUiState(state.uiState),
    };
}
function serializeUiState(state) {
    return {
        mode: state.mode,
        bounds: {
            x: state.x,
            y: state.y,
            width: state.width,
            height: state.height,
        },
        ...(state.displayId === undefined ? {} : { displayId: state.displayId }),
    };
}
function parseWindowsState(value) {
    if (!isRecord(value) ||
        value.version !== WINDOWS_STATE_VERSION ||
        !Array.isArray(value.openedWindows)) {
        return { openedWindows: [] };
    }
    const lastActiveWindow = value.lastActiveWindow === undefined
        ? undefined
        : parseWindowStateRecord(value.lastActiveWindow);
    const openedWindows = value.openedWindows
        .map(parseWindowStateRecord)
        .filter((state) => state !== undefined);
    return {
        ...(lastActiveWindow === undefined ? {} : { lastActiveWindow }),
        openedWindows,
    };
}
function parseWindowStateRecord(value) {
    if (!isRecord(value)) {
        return undefined;
    }
    const uiState = parseUiState(value.uiState);
    if (!uiState) {
        return undefined;
    }
    const identityCount = Number(value.workspaceIdentifier !== undefined) +
        Number(value.folder !== undefined) +
        Number(value.backupPath !== undefined);
    if (identityCount > 1) {
        return undefined;
    }
    if (value.workspaceIdentifier !== undefined) {
        const workspace = parseStoredWorkspace(value.workspaceIdentifier);
        return workspace ? { workspace, uiState } : undefined;
    }
    if (value.folder !== undefined) {
        const folderUri = parseFileUri(value.folder);
        return folderUri ? { folderUri, uiState } : undefined;
    }
    if (value.backupPath !== undefined) {
        return isNonEmptyString(value.backupPath)
            ? { backupPath: value.backupPath, uiState }
            : undefined;
    }
    return { uiState };
}
function parseStoredWorkspace(value) {
    if (!isRecord(value) || !isNonEmptyString(value.id)) {
        return undefined;
    }
    const configPath = parseFileUri(value.configURIPath);
    return configPath
        ? Object.freeze({ id: value.id, configPath })
        : undefined;
}
function parseFileUri(value) {
    if (typeof value !== "string") {
        return undefined;
    }
    try {
        const uri = URI.parse(value);
        return uri.scheme === "file" && !uri.query && !uri.fragment
            ? uri
            : undefined;
    }
    catch {
        return undefined;
    }
}
function parseUiState(value) {
    if (!isRecord(value)) {
        return undefined;
    }
    const mode = value.mode;
    const storedBounds = value.bounds;
    if (!isWindowMode(mode) || !isRecord(storedBounds)) {
        return undefined;
    }
    const bounds = readBounds(() => ({
        x: storedBounds.x,
        y: storedBounds.y,
        width: storedBounds.width,
        height: storedBounds.height,
    }));
    if (!bounds) {
        return undefined;
    }
    if (value.displayId !== undefined &&
        !isFiniteNumber(value.displayId)) {
        return undefined;
    }
    return {
        mode,
        ...bounds,
        displayId: value.displayId,
    };
}
function toBounds(state) {
    if (!isFiniteNumber(state.x) || !isFiniteNumber(state.y)) {
        return undefined;
    }
    return {
        x: state.x,
        y: state.y,
        width: state.width,
        height: state.height,
    };
}
function readBounds(read) {
    try {
        const bounds = read();
        if (!isFiniteNumber(bounds.x) ||
            !isFiniteNumber(bounds.y) ||
            !isFiniteNumber(bounds.width) ||
            !isFiniteNumber(bounds.height) ||
            bounds.width <= 0 ||
            bounds.height <= 0) {
            return undefined;
        }
        return {
            x: bounds.x,
            y: bounds.y,
            width: bounds.width,
            height: bounds.height,
        };
    }
    catch {
        return undefined;
    }
}
function isRecord(value) {
    return typeof value === "object" && value !== null && !Array.isArray(value);
}
function isWindowMode(value) {
    return value === WindowMode.Normal ||
        value === WindowMode.Maximized ||
        value === WindowMode.Fullscreen;
}
function isFiniteNumber(value) {
    return typeof value === "number" && Number.isFinite(value);
}
function isNonEmptyString(value) {
    return typeof value === "string" && value.trim().length > 0;
}
