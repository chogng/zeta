import { WorkbenchPart } from "../../part.js";
import { Viewlet } from "../views/viewlet.js";

/** The primary navigation and view-container region on the workbench side. */
export class SidebarPart extends WorkbenchPart {
  constructor() {
    super("sidebar");
    this.element.setAttribute("aria-label", "Primary sidebar");
  }

  setContent(content: Element): void { this.contentElement.replaceChildren(content); }
  setViewlet(viewlet: Viewlet): void { this.setContent(viewlet.element); }
}
