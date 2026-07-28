import {
  DisposableOwner,
} from "../../../common/lifecycle.js";
import { observeElementSize } from "../../observer.js";

/** A browser-native resize surface that notifies consumers after layout changes. */
export class Resizable extends DisposableOwner {
  readonly element: HTMLDivElement;

  constructor(onResize?: (size: { width: number; height: number }) => void) {
    super();
    const element = document.createElement("div");
    this.element = element;
    this.defer(() => element.remove());
    element.className = "zeta-resizable";
    element.style.resize = "both";
    element.style.overflow = "auto";
    this.own(observeElementSize(element, (size) => onResize?.(size)));
  }
}
