import {
  DEFAULT_WINDOW_SIZE,
} from "../common/window.js";

export const WindowMode = {
  Normal: "normal",
  Maximized: "maximized",
  Fullscreen: "fullscreen",
} as const;

/** Stable string values used by runtime and persisted window state. */
export type WindowMode = typeof WindowMode[keyof typeof WindowMode];

/** A complete operating-system window rectangle. */
export interface IWindowBounds {
  readonly x: number;
  readonly y: number;
  readonly width: number;
  readonly height: number;
}

/**
 * Runtime state used to create and restore an Electron window.
 *
 * A default state may omit its position so the operating system can choose it.
 * Restored states contain both `x` and `y` after validation.
 */
export interface IWindowState {
  readonly mode: WindowMode;
  readonly width: number;
  readonly height: number;
  readonly x?: number;
  readonly y?: number;
  readonly displayId?: number;
}

/** Returns a fresh default state for a new main window. */
export function defaultWindowState(
  mode: WindowMode = WindowMode.Normal,
): IWindowState {
  return {
    mode,
    width: DEFAULT_WINDOW_SIZE.width,
    height: DEFAULT_WINDOW_SIZE.height,
  };
}
