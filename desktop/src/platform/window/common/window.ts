/** Dimensions used when no valid saved window state is available. */
export const DEFAULT_WINDOW_SIZE = {
  width: 1200,
  height: 800,
} as const;

/** Lower bounds that keep the workbench usable while resizing. */
export const WINDOW_MINIMUM_SIZE = {
  width: 400,
  height: 270,
} as const;
