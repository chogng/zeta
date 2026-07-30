import "./auxiliarybarpart.css";
import { DisposableSlot } from "../../../../base/common/lifecycle.js";
import { WorkbenchPart } from "../../part.js";
import { ViewPaneContainer } from "../views/viewPaneContainer.js";

/**
 * Fixed secondary region populated by an Auxiliary feature contribution.
 *
 * The hosted feature owns its title navigation and actions. This Part owns
 * only Workbench layout, visibility, and the container lifetime.
 */
export class AuxiliarybarPart extends WorkbenchPart {
  private readonly viewPaneContainer =
    this.own(new DisposableSlot<ViewPaneContainer>());

  override get minimumWidth(): number { return 180; }
  override get maximumWidth(): number { return 600; }

  constructor(ownerDocument: Document) {
    super("auxiliarybar", ownerDocument);
    this.element.setAttribute("aria-label", "Auxiliary sidebar");
    this.titleElement.remove();
    this.contentElement.classList.add("zeta-auxiliarybar-content");
  }

  setViewPaneContainer(container: ViewPaneContainer): void {
    this.viewPaneContainer.replace(container);
    this.contentElement.replaceChildren(container.element);
  }
}
