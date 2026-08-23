import { type IDisposable, toDisposable } from "../common/lifecycle.js";

export type DomListenerOptions = boolean | AddEventListenerOptions;
export type DomChild = Node | string;
export type DomTreeChild =
  | Node
  | string
  | number
  | false
  | null
  | undefined
  | readonly DomTreeChild[];

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

/** Cross-realm Element guard that does not rely on global instanceof. */
export function isElement(value: unknown): value is Element {
  return isNode(value) && (value as Node).nodeType === 1;
}

/** Cross-realm HTMLElement guard that does not rely on global instanceof. */
export function isHTMLElement(value: unknown): value is HTMLElement {
  return isElement(value) &&
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

type PrimitiveDomProperty = string | number | boolean | null | undefined;

type SafeDomPropertyName<TElement> = {
  [TKey in keyof TElement]-?: TKey extends "innerHTML" | "outerHTML"
    ? never
    : TElement[TKey] extends PrimitiveDomProperty ? TKey
    : never;
}[keyof TElement];

export type DomElementProperties<TElement> = Partial<
  Pick<TElement, SafeDomPropertyName<TElement>>
>;

export interface DomElementOptions<TElement extends HTMLElement | SVGElement> {
  readonly className?: string | readonly (string | false | null | undefined)[];
  /** String-valued markup attributes. Use `properties` for native booleans. */
  readonly attributes?: Readonly<
    Record<string, string | number | null | undefined>
  >;
  readonly properties?: Readonly<DomElementProperties<TElement>>;
  readonly dataset?: Readonly<Record<string, string | undefined>>;
  /** Camel-case or CSS property names map to CSS text. Length units must be explicit. */
  readonly style?: Readonly<Record<string, string | undefined>>;
  readonly ref?: (element: TElement) => void;
}

export type HtmlElementForTag<TTag extends string> =
  TTag extends keyof HTMLElementTagNameMap ? HTMLElementTagNameMap[TTag]
    : HTMLElement;

/**
 * Document-bound DOM construction function.
 *
 * Binding construction to a document keeps auxiliary windows and test DOMs
 * isolated without relying on the process-global `document`.
 */
export interface DomFactory {
  readonly document: Document;

  <TTag extends string>(
    tag: TTag,
    optionsOrChild?: DomElementOptions<HtmlElementForTag<TTag>> | DomTreeChild,
    ...children: readonly DomTreeChild[]
  ): HtmlElementForTag<TTag>;

  svg<K extends keyof SVGElementTagNameMap>(
    tag: K,
    optionsOrChild?: DomElementOptions<SVGElementTagNameMap[K]> | DomTreeChild,
    ...children: readonly DomTreeChild[]
  ): SVGElementTagNameMap[K];

  text(value: string | number): Text;
  fragment(...children: readonly DomTreeChild[]): DocumentFragment;
}

/** Creates a callable DOM factory bound to `ownerDocument`. */
export function createDom(ownerDocument: Document): DomFactory {
  const factory = (<TTag extends string>(
    tag: TTag,
    optionsOrChild?: DomElementOptions<HtmlElementForTag<TTag>> | DomTreeChild,
    ...children: readonly DomTreeChild[]
  ): HtmlElementForTag<TTag> =>
    h(ownerDocument, tag, optionsOrChild, ...children)) as DomFactory;

  Object.defineProperties(factory, {
    document: { value: ownerDocument, enumerable: true },
    svg: {
      value: <K extends keyof SVGElementTagNameMap>(
        tag: K,
        optionsOrChild?: DomElementOptions<SVGElementTagNameMap[K]> | DomTreeChild,
        ...children: readonly DomTreeChild[]
      ): SVGElementTagNameMap[K] =>
        svg(ownerDocument, tag, optionsOrChild, ...children),
      enumerable: true,
    },
    text: {
      value: (value: string | number): Text =>
        text(ownerDocument, value),
      enumerable: true,
    },
    fragment: {
      value: (...children: readonly DomTreeChild[]): DocumentFragment => {
        return fragment(ownerDocument, ...children);
      },
      enumerable: true,
    },
  });

  return factory;
}

/** Creates one HTML element in the supplied document. */
export function h<TTag extends string>(
  ownerDocument: Document,
  tag: TTag,
  optionsOrChild?: DomElementOptions<HtmlElementForTag<TTag>> | DomTreeChild,
  ...children: readonly DomTreeChild[]
): HtmlElementForTag<TTag> {
  const args = splitElementArguments(optionsOrChild, children);
  const element = ownerDocument.createElement(tag) as HtmlElementForTag<TTag>;
  applyElementOptions(element, args.options);
  appendDomChildren(element, args.children);
  args.options.ref?.(element);
  return element;
}

/** Creates one SVG element in the supplied document. */
export function svg<K extends keyof SVGElementTagNameMap>(
  ownerDocument: Document,
  tag: K,
  optionsOrChild?: DomElementOptions<SVGElementTagNameMap[K]> | DomTreeChild,
  ...children: readonly DomTreeChild[]
): SVGElementTagNameMap[K] {
  const args = splitElementArguments(optionsOrChild, children);
  const element = ownerDocument.createElementNS(
    "http://www.w3.org/2000/svg",
    tag,
  );
  applyElementOptions(element, args.options);
  appendDomChildren(element, args.children);
  args.options.ref?.(element);
  return element;
}

/** Creates a text node in the supplied document. */
export function text(
  ownerDocument: Document,
  value: string | number,
): Text {
  return ownerDocument.createTextNode(String(value));
}

/** Creates a document fragment containing the supplied children. */
export function fragment(
  ownerDocument: Document,
  ...children: readonly DomTreeChild[]
): DocumentFragment {
  const result = ownerDocument.createDocumentFragment();
  appendDomChildren(result, children);
  return result;
}

function applyElementOptions<TElement extends HTMLElement | SVGElement>(
  element: TElement,
  options: DomElementOptions<TElement>,
): void {
  const classNames = typeof options.className === "string"
    ? options.className.split(/\s+/)
    : options.className;
  if (classNames) {
    element.classList.add(...classNames.filter((name): name is string => Boolean(name)));
  }
  for (const [name, value] of Object.entries(options.attributes ?? {})) {
    if (value === null || value === undefined) element.removeAttribute(name);
    else element.setAttribute(name, String(value));
  }
  for (const [name, value] of Object.entries(options.properties ?? {})) {
    Reflect.set(element, name, value);
  }
  if (element.namespaceURI === "http://www.w3.org/1999/xhtml") {
    const htmlElement = element as HTMLElement;
    for (const [name, value] of Object.entries(options.dataset ?? {})) {
      if (value === undefined) delete htmlElement.dataset[name];
      else htmlElement.dataset[name] = value;
    }
  }
  for (const [name, value] of Object.entries(options.style ?? {})) {
    const propertyName = toCssPropertyName(name);
    if (value === undefined) element.style.removeProperty(propertyName);
    else element.style.setProperty(propertyName, value);
  }
}

function isDomElementOptions<TElement extends HTMLElement | SVGElement>(
  value: DomElementOptions<TElement> | DomTreeChild,
): value is DomElementOptions<TElement> {
  return typeof value === "object" && value !== null && !Array.isArray(value) && !isNode(value);
}

function splitElementArguments<TElement extends HTMLElement | SVGElement>(
  optionsOrChild: DomElementOptions<TElement> | DomTreeChild,
  children: readonly DomTreeChild[],
): {
  readonly options: DomElementOptions<TElement>;
  readonly children: readonly DomTreeChild[];
} {
  return isDomElementOptions(optionsOrChild)
    ? { options: optionsOrChild, children }
    : { options: {}, children: optionsOrChild === undefined ? children : [optionsOrChild, ...children] };
}

function toCssPropertyName(name: string): string {
  return name.startsWith("--") ? name : name.replace(/[A-Z]/g, letter => `-${letter.toLowerCase()}`);
}

function appendDomChildren(
  parent: Element | DocumentFragment,
  children: readonly DomTreeChild[],
): void {
  for (const child of children) {
    if (Array.isArray(child)) {
      appendDomChildren(parent, child);
    } else if (child !== false && child !== null && child !== undefined) {
      parent.append(
        typeof child === "string" || typeof child === "number"
          ? String(child)
          : child as Node,
      );
    }
  }
}
