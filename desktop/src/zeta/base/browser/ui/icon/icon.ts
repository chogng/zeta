import { type Icon, type IconDefinition, resolveIconDefinition } from "../../../common/icon.js";
import { setAriaAttribute } from "../aria/aria.js";

const iconPrototypesByDocument = new WeakMap<Document, Map<IconDefinition, SVGElement>>();

/** Renders an icon reference with consistent accessibility metadata. */
export function appendIcon(icon: Icon, container: HTMLElement): SVGElement {
  const element = iconPrototype(icon, container.ownerDocument).cloneNode(true) as SVGElement;
  container.append(element);
  return element;
}

function iconPrototype(icon: Icon, document: Document): SVGElement {
  const definition = resolveIconDefinition(icon);
  let prototypes = iconPrototypesByDocument.get(document);
  if (!prototypes) {
    prototypes = new Map();
    iconPrototypesByDocument.set(document, prototypes);
  }
  const existing = prototypes.get(definition);
  if (existing) {
    return existing;
  }

  const template = document.createElement("template");
  template.innerHTML = definition().trim();
  const candidate = template.content.firstElementChild;
  if (template.content.childElementCount !== 1 || candidate?.namespaceURI !== "http://www.w3.org/2000/svg") {
    throw new TypeError(`Icon '${icon.id}' did not produce one SVG element`);
  }

  const prototype = candidate as SVGElement;
  prototype.classList.add("zeta-icon");
  setAriaAttribute(prototype, "hidden", true);
  prototype.setAttribute("focusable", "false");
  prototypes.set(definition, prototype);
  return prototype;
}
