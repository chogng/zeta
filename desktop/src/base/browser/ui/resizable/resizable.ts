import { Component } from "../common/component.js";

/** A browser-native resize surface that notifies consumers after layout changes. */
export class Resizable extends Component<HTMLDivElement> {
  #observer: ResizeObserver;

  constructor(onResize?: (size: { width: number; height: number }) => void) {
    const element = document.createElement("div");
    element.className = "zeta-resizable";
    element.style.resize = "both";
    element.style.overflow = "auto";
    super(element);
    this.#observer = new ResizeObserver(([entry]) => onResize?.({ width: entry.contentRect.width, height: entry.contentRect.height }));
    this.#observer.observe(element);
  }

  override dispose(): void { this.#observer.disconnect(); super.dispose(); }
}
