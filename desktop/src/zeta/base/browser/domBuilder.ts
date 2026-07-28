export type DomBuilderChild =
  | Node
  | string
  | number
  | false
  | null
  | undefined
  | readonly DomBuilderChild[];

export interface DomElementOptions<TElement extends HTMLElement | SVGElement> {
  readonly className?: string | readonly string[];
  readonly attributes?: Readonly<
    Record<string, string | number | boolean | null | undefined>
  >;
  readonly dataset?: Readonly<Record<string, string | undefined>>;
  readonly style?: Readonly<Record<string, string | number | undefined>>;
  readonly ref?: (element: TElement) => void;
}

/**
 * DOM construction helper bound to one Document so elements are created in
 * the correct browser window.
 */
export class DomBuilder {
  constructor(readonly document: Document) {}

  element<K extends keyof HTMLElementTagNameMap>(
    tag: K,
    options: DomElementOptions<HTMLElementTagNameMap[K]> = {},
    ...children: readonly DomBuilderChild[]
  ): HTMLElementTagNameMap[K] {
    const element = this.document.createElement(tag);
    applyOptions(element, options);
    appendChildren(element, children);
    options.ref?.(element);
    return element;
  }

  svg<K extends keyof SVGElementTagNameMap>(
    tag: K,
    options: DomElementOptions<SVGElementTagNameMap[K]> = {},
    ...children: readonly DomBuilderChild[]
  ): SVGElementTagNameMap[K] {
    const element = this.document.createElementNS(
      "http://www.w3.org/2000/svg",
      tag,
    );
    applyOptions(element, options);
    appendChildren(element, children);
    options.ref?.(element);
    return element;
  }

  text(value: string | number): Text {
    return this.document.createTextNode(String(value));
  }
}

export function createDomBuilder(ownerDocument: Document): DomBuilder {
  return new DomBuilder(ownerDocument);
}

function applyOptions<TElement extends HTMLElement | SVGElement>(
  element: TElement,
  options: DomElementOptions<TElement>,
): void {
  const classNames = typeof options.className === "string"
    ? options.className.split(/\s+/)
    : options.className;
  if (classNames) {
    element.classList.add(...classNames.filter(Boolean));
  }
  for (const [name, value] of Object.entries(options.attributes ?? {})) {
    if (value === false || value === null || value === undefined) {
      element.removeAttribute(name);
    } else {
      element.setAttribute(name, value === true ? "" : String(value));
    }
  }
  if (element.namespaceURI === "http://www.w3.org/1999/xhtml") {
    const htmlElement = element as HTMLElement;
    for (const [name, value] of Object.entries(options.dataset ?? {})) {
      if (value === undefined) delete htmlElement.dataset[name];
      else htmlElement.dataset[name] = value;
    }
  }
  for (const [name, value] of Object.entries(options.style ?? {})) {
    if (value === undefined) {
      element.style.removeProperty(name);
    } else {
      element.style.setProperty(
        name,
        typeof value === "number" ? `${value}px` : value,
      );
    }
  }
}

function appendChildren(
  parent: Element,
  children: readonly DomBuilderChild[],
): void {
  for (const child of children) {
    if (Array.isArray(child)) {
      appendChildren(parent, child);
    } else if (
      child !== false &&
      child !== null &&
      child !== undefined
    ) {
      parent.append(
        typeof child === "string" || typeof child === "number"
          ? String(child)
          : child as Node,
      );
    }
  }
}
