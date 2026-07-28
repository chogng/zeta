import type { IDisposable } from "../../../common/lifecycle.js";

/**
 * A disposable browser view that exposes the element its host should attach.
 */
export interface IElementView extends IDisposable {
  readonly element: HTMLElement;
}
