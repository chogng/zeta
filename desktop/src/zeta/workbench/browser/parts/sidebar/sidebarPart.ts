import "./sidebarpart.css";
import { WorkbenchPart } from "../../part.js";
import { DisposableSlot } from "../../../../base/common/lifecycle.js";
import type { ActivitybarPart } from "../activitybar/activitybarPart.js";
import { Viewlet } from "../views/viewlet.js";

/** The primary navigation and view-container region on the workbench side. */
export class SidebarPart extends WorkbenchPart {
  readonly #viewlet: DisposableSlot<Viewlet>;
  readonly #titleLabel: HTMLHeadingElement;

  override get minimumWidth(): number { return 180; }
  override get maximumWidth(): number { return 600; }

  constructor(
    ownerDocument: Document,
    activitybar: ActivitybarPart,
  ) {
    super("sidebar", ownerDocument);
    this.#viewlet = this.own(new DisposableSlot<Viewlet>());
    this.element.setAttribute("aria-label", "Primary sidebar");
    this.element.prepend(activitybar.element);
    this.#titleLabel = ownerDocument.createElement("h2");
    this.#titleLabel.className = "zeta-sidebar-title";
    this.titleElement.append(this.#titleLabel);
  }

  setContent(content: Element): void {
    this.#viewlet.clear();
    this.#titleLabel.textContent = "";
    this.contentElement.replaceChildren(content);
  }

  setViewlet(viewlet: Viewlet): void {
    this.#viewlet.replace(viewlet);
    this.#titleLabel.textContent = viewlet.title;
    this.contentElement.replaceChildren(viewlet.element);
  }

  get activeViewletId(): string | undefined {
    return this.#viewlet.value?.id;
  }
}
