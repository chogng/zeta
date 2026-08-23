import { type IDisposable, toDisposable } from "../common/lifecycle.js";

type DomListenerOptions = boolean | AddEventListenerOptions;
type DomChild = Node | string;
type DomTreeChild =
	| Node
	| string
	| number
	| false
	| null
	| undefined
	| readonly DomTreeChild[];

const DOCUMENT_NODE_TYPE = 9;
const ELEMENT_NODE_TYPE = 1;
const HTML_NAMESPACE = "http://www.w3.org/1999/xhtml";

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
	const capture = typeof options === "boolean" ? options : options?.capture ?? false;
	let activeTarget: EventTarget | undefined = target;
	let activeListener: EventListener | undefined = eventListener;
	return toDisposable(() => {
		if (activeTarget && activeListener) {
			activeTarget.removeEventListener(type, activeListener, capture);
		}
		activeTarget = undefined;
		activeListener = undefined;
	});
}

/** Replaces a container's children with the supplied nodes or text. */
export function reset(
	parent: Element | DocumentFragment,
	...children: readonly DomChild[]
): void {
	parent.replaceChildren(...children);
}

/** Tests DOM ancestry without assuming that either node is connected. */
export function isAncestor(
	candidate: Node | null,
	ancestor: Node | null,
): boolean {
	return Boolean(candidate && ancestor?.contains(candidate));
}

export function isNode(value: unknown): value is Node {
	if (typeof value !== "object" || value === null) {
		return false;
	}
	const nodeConstructor = getNodeConstructor(value);
	if (nodeConstructor) {
		return value instanceof nodeConstructor;
	}
	return typeof Node !== "undefined" && value instanceof Node;
}

/** Cross-realm Element guard. */
export function isElement(value: unknown): value is Element {
	return isNode(value) && value.nodeType === ELEMENT_NODE_TYPE;
}

/** Cross-realm HTMLElement guard. */
export function isHTMLElement(value: unknown): value is HTMLElement {
	if (!isElement(value)) {
		return false;
	}
	const htmlElementConstructor = value.ownerDocument?.defaultView?.HTMLElement;
	return typeof htmlElementConstructor === "function"
		? value instanceof htmlElementConstructor
		: value.namespaceURI === HTML_NAMESPACE;
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

interface DomElementOptions<TElement extends HTMLElement | SVGElement> {
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

type HtmlElementForTag<TTag extends string> =
	TTag extends keyof HTMLElementTagNameMap ? HTMLElementTagNameMap[TTag]
		: HTMLElement;

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
	for (const [name, value] of Object.entries(options.dataset ?? {})) {
		if (value === undefined) {
			delete element.dataset[name];
		} else {
			element.dataset[name] = value;
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

function getNodeConstructor(value: object): typeof Node | undefined {
	const candidate = value as Node;
	const ownerDocument = candidate.nodeType === DOCUMENT_NODE_TYPE
		? value as Document
		: candidate.ownerDocument;
	const nodeConstructor = ownerDocument?.defaultView?.Node;
	return typeof nodeConstructor === "function" ? nodeConstructor : undefined;
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
