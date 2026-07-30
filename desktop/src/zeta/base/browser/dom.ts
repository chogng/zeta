import {
  type IDisposable,
  toDisposable,
} from "../common/lifecycle.js";

export type DomListenerOptions = boolean | AddEventListenerOptions;
export type DomChild = Node | string;

/**
 * Registers a DOM event listener whose removal participates in disposable
 * ownership.
 */
export function addDisposableListener<
  K extends keyof GlobalEventHandlersEventMap,
>(
  target: EventTarget,
  type: K,
  listener: (event: GlobalEventHandlersEventMap[K]) => void,
  options?: DomListenerOptions,
): IDisposable;
export function addDisposableListener<TEvent extends Event>(
  target: EventTarget,
  type: string,
  listener: (event: TEvent) => void,
  options?: DomListenerOptions,
): IDisposable;
export function addDisposableListener(
  target: EventTarget,
  type: string,
  listener: (event: Event) => void,
  options?: DomListenerOptions,
): IDisposable {
  const eventListener = listener as EventListener;
  target.addEventListener(type, eventListener, options);
  let activeTarget: EventTarget | undefined = target;
  let activeListener: EventListener | undefined = eventListener;
  return toDisposable(() => {
    if (activeTarget && activeListener) {
      activeTarget.removeEventListener(type, activeListener, options);
    }
    activeTarget = undefined;
    activeListener = undefined;
  });
}

/** Removes every child from a DOM container. */
export function clearNode<T extends Element | DocumentFragment>(node: T): T {
  node.replaceChildren();
  return node;
}

/** Appends a child and returns it for fluent construction. */
export function append<T extends Node>(
  parent: Element | DocumentFragment,
  child: T,
): T;
export function append(
  parent: Element | DocumentFragment,
  ...children: readonly DomChild[]
): void;
export function append<T extends Node>(
  parent: Element | DocumentFragment,
  ...children: readonly DomChild[]
): T | void {
  parent.append(...children);
  return children.length === 1 && typeof children[0] !== "string"
    ? children[0] as T
    : undefined;
}

/** Replaces a container's children with the supplied nodes or text. */
export function reset(
  parent: Element | DocumentFragment,
  ...children: readonly DomChild[]
): void {
  parent.replaceChildren(...children);
}

/** Updates native visibility without overwriting layout-related styles. */
export function setVisibility(
  visible: boolean,
  ...elements: readonly HTMLElement[]
): void {
  for (const element of elements) element.hidden = !visible;
}

export function show(...elements: readonly HTMLElement[]): void {
  setVisibility(true, ...elements);
}

export function hide(...elements: readonly HTMLElement[]): void {
  setVisibility(false, ...elements);
}

/** Tests DOM ancestry without assuming that either node is connected. */
export function isAncestor(
  candidate: Node | null,
  ancestor: Node | null,
): boolean {
  return Boolean(candidate && ancestor?.contains(candidate));
}

export function isNode(value: unknown): value is Node {
  return typeof value === "object" &&
    value !== null &&
    typeof (value as Node).nodeType === "number";
}

/** Cross-realm HTMLElement guard that does not rely on global instanceof. */
export function isHTMLElement(value: unknown): value is HTMLElement {
  return isNode(value) &&
    (value as Node).nodeType === 1 &&
    (value as Element).namespaceURI === "http://www.w3.org/1999/xhtml";
}

export function isHTMLInputElement(
  value: unknown,
): value is HTMLInputElement {
  return isHTMLElement(value) && value.tagName === "INPUT";
}

export function isHTMLButtonElement(
  value: unknown,
): value is HTMLButtonElement {
  return isHTMLElement(value) && value.tagName === "BUTTON";
}

/** Stops propagation and, by default, the browser's native behavior. */
export function stopEvent(
  event: Event,
  options: {
    readonly preventDefault?: boolean;
    readonly immediate?: boolean;
  } = {},
): void {
  if (options.preventDefault !== false) event.preventDefault();
  if (options.immediate) event.stopImmediatePropagation();
  else event.stopPropagation();
}
