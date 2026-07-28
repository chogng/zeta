import {
  lxAdd,
  lxStart,
} from "@chogng/lxicons";
import { registerIcon } from "./icon.js";

/**
 * Default icons supplied by Lxicons.
 *
 * Keep vendor imports in this library instead of exposing SVG factories to
 * controls and product code. Add entries here only as the application uses
 * them so the renderer bundle remains tree-shakable.
 */
export const LxIcon = {
  add: registerIcon("add", lxAdd),
  start: registerIcon("start", lxStart),
} as const;
