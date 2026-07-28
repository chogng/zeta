import { WorkbenchPart } from "../../part.js";
import { DisposableSlot } from "../../../../base/common/lifecycle.js";
import { Viewlet } from "../views/viewlet.js";

/** The primary navigation and view-container region on the workbench side. */
export class SidebarPart extends WorkbenchPart {
  readonly #viewlet: DisposableSlot<Viewlet>;

  override get minimumWidth(): number { return 180; }
  override get maximumWidth(): number { return 600; }

  constructor(ownerDocument: Document) {
    super("sidebar", ownerDocument);
    this.#viewlet = this.own(new DisposableSlot<Viewlet>());
    this.element.setAttribute("aria-label", "Primary sidebar");
  }

  setContent(content: Element): void {
    this.#viewlet.clear();
    this.contentElement.replaceChildren(content);
  }

  setViewlet(viewlet: Viewlet): void {
    this.#viewlet.replace(viewlet);
    this.contentElement.replaceChildren(viewlet.element);
  }
}
