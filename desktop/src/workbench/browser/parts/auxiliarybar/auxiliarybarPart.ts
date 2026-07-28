import { WorkbenchPart } from "../../part.js";
import { DisposableSlot } from "../../../../base/common/lifecycle.js";
import { ViewPaneContainer } from "../views/viewPaneContainer.js";

/** An optional secondary side region for contextual tools and inspectors. */
export class AuxiliarybarPart extends WorkbenchPart {
  readonly #viewPaneContainer: DisposableSlot<ViewPaneContainer>;

  constructor(ownerDocument: Document) {
    super("auxiliarybar", ownerDocument);
    this.#viewPaneContainer = this.own(
      new DisposableSlot<ViewPaneContainer>(),
    );
    this.element.setAttribute("aria-label", "Auxiliary sidebar");
  }

  setContent(content: Element): void {
    this.#viewPaneContainer.clear();
    this.contentElement.replaceChildren(content);
  }

  setViewPaneContainer(container: ViewPaneContainer): void {
    this.#viewPaneContainer.replace(container);
    this.contentElement.replaceChildren(container.element);
  }
}
