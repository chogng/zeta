import { ACADEMIC_NODE_TYPES } from "../common/schema.js";
import type { GamaNodeViewContext, GamaNodeViewFactory } from "../../../browser/gamaEditorSession.js";

/** Browser projections for Academic wrapper nodes; child editing stays Gama-owned. */
export const nodeViews: Readonly<Record<string, GamaNodeViewFactory>> = Object.freeze({
  [ACADEMIC_NODE_TYPES.title]: context => createWrapper(context, "header", "title", "Document title"),
  [ACADEMIC_NODE_TYPES.abstract]: context => createWrapper(context, "section", "abstract", "Abstract"),
  [ACADEMIC_NODE_TYPES.section]: context => createWrapper(context, "section", "section"),
});

function createWrapper(context: GamaNodeViewContext, tagName: "header" | "section", role: string, label?: string): HTMLElement {
  const element = context.previousElement ?? context.ownerDocument.createElement(tagName);
  element.className = "zeta-academic-" + role;
  element.dataset.academicRole = role;
  if (label) element.setAttribute("aria-label", label);
  context.renderChildren(element);
  return element;
}
