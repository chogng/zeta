import "./sidebarpart.css";
import type { ActivitybarPart } from "../activitybar/activitybarPart.js";
import { CompositePart } from "../compositePart.js";

/** Primary CompositePart presented at the side of the workbench. */
export class SidebarPart extends CompositePart {
  override get minimumWidth(): number { return 180; }
  override get maximumWidth(): number { return 600; }

  constructor(
    ownerDocument: Document,
    activitybar: ActivitybarPart,
  ) {
    super("sidebar", ownerDocument);
    this.element.setAttribute("aria-label", "Primary sidebar");
    this.element.prepend(activitybar.element);
  }
}
