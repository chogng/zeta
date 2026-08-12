import { type Event } from "../../../base/common/event.js";
import { type IDisposable } from "../../../base/common/lifecycle.js";

/** Optional feature-owned projection hosted in one rendered line's gutter slot. */
export interface EditorLineGutterDecoration extends IDisposable {
  readonly onDidChange: Event<void>;
  create(ownerDocument: Document): HTMLElement;
  project(element: HTMLElement, logicalLineIndex: number, firstForLogicalLine: boolean): void;
}
