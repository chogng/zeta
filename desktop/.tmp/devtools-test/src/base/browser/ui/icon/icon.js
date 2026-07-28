import { resolveIconDefinition, } from "../../../common/icon.js";
import { setAriaAttribute } from "../aria/aria.js";
/** Renders an icon reference with consistent accessibility metadata. */
export function appendIcon(icon, container) {
    const template = container.ownerDocument.createElement("template");
    template.innerHTML = resolveIconDefinition(icon)().trim();
    const candidate = template.content.firstElementChild;
    if (template.content.childElementCount !== 1 ||
        candidate?.namespaceURI !== "http://www.w3.org/2000/svg") {
        throw new TypeError(`Icon '${icon.id}' did not produce one SVG element`);
    }
    const element = candidate;
    element.classList.add("zeta-icon");
    setAriaAttribute(element, "hidden", true);
    element.setAttribute("focusable", "false");
    container.append(element);
    return element;
}
