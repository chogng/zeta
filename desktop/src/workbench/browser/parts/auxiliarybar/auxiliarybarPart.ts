import { WorkbenchPart } from "../../part.js";
import { ViewPaneContainer } from "../views/viewPaneContainer.js";

/** An optional secondary side region for contextual tools and inspectors. */
export class AuxiliarybarPart extends WorkbenchPart {
  constructor() {
    super("auxiliarybar");
    this.element.setAttribute("aria-label", "Auxiliary sidebar");
  }

  setContent(content: Element): void { this.contentElement.replaceChildren(content); }
  setViewPaneContainer(container: ViewPaneContainer): void { this.setContent(container.element); }
}
