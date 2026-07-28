import {
  DEFAULT_EMPTY_WINDOW_SIZE,
  DEFAULT_WORKSPACE_WINDOW_SIZE,
  WindowKind,
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

/** Returns a fresh default state sized for the requested workbench kind. */
export function defaultWindowState(
  kind: WindowKind,
  mode: WindowMode = WindowMode.Normal,
): IWindowState {
  const size = kind === WindowKind.Workspace
    ? DEFAULT_WORKSPACE_WINDOW_SIZE
    : DEFAULT_EMPTY_WINDOW_SIZE;

  return {
    mode,
    width: size.width,
    height: size.height,
  };
}
