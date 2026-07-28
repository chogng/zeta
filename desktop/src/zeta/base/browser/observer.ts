import {
  type IDisposable,
  toDisposable,
} from "../common/lifecycle.js";
import { Dimension } from "./geometry.js";

/** Observes element resize records until the returned registration is disposed. */
export function observeResize(
  target: Element,
  listener: (entries: readonly ResizeObserverEntry[]) => void,
  options?: ResizeObserverOptions,
): IDisposable {
  const observer = new ResizeObserver((entries) => listener(entries));
  observer.observe(target, options);
  return toDisposable(() => observer.disconnect());
}

/** Observes the border-box size of one element. */
export function observeElementSize(
  target: HTMLElement,
  listener: (size: Dimension) => void,
): IDisposable {
  return observeResize(target, ([entry]) => {
    if (!entry) return;
    listener(new Dimension(
      entry.borderBoxSize[0]?.inlineSize ?? entry.contentRect.width,
      entry.borderBoxSize[0]?.blockSize ?? entry.contentRect.height,
    ));
  }, { box: "border-box" });
}

export function observeMutations(
  target: Node,
  listener: (records: readonly MutationRecord[]) => void,
  options: MutationObserverInit,
): IDisposable {
  const observer = new MutationObserver((records) => listener(records));
  observer.observe(target, options);
  return toDisposable(() => observer.disconnect());
}

export function observeIntersection(
  target: Element,
  listener: (entry: IntersectionObserverEntry) => void,
  options?: IntersectionObserverInit,
): IDisposable {
  const observer = new IntersectionObserver(([entry]) => {
    if (entry) listener(entry);
  }, options);
  observer.observe(target);
  return toDisposable(() => observer.disconnect());
}
