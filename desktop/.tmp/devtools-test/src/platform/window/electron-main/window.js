import { DEFAULT_EMPTY_WINDOW_SIZE, DEFAULT_WORKSPACE_WINDOW_SIZE, } from "../common/window.js";
export const WindowMode = {
    Normal: "normal",
    Maximized: "maximized",
    Fullscreen: "fullscreen",
};
/** Returns a fresh default state sized for the requested workbench state. */
export function defaultWindowState(workbenchState, mode = WindowMode.Normal) {
    const size = workbenchState === 1 /* WorkbenchState.EMPTY */
        ? DEFAULT_EMPTY_WINDOW_SIZE
        : DEFAULT_WORKSPACE_WINDOW_SIZE;
    return {
        mode,
        width: size.width,
        height: size.height,
    };
}
