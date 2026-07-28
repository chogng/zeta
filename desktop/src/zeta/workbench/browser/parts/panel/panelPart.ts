import "./panelpart.css";
import { DisposableSlot } from "../../../../base/common/lifecycle.js";
import { WorkbenchPart } from "../../part.js";
import { ViewPaneContainer } from "../views/viewPaneContainer.js";

/** The bottom tool region that hosts terminals and other panel views. */
export class PanelPart extends WorkbenchPart {
  readonly #viewPaneContainer =
    this.own(new DisposableSlot<ViewPaneContainer>());

  override get minimumHeight(): number { return 80; }

  constructor(ownerDocument: Document) {
    super("panel", ownerDocument);
    this.element.setAttribute("aria-label", "Panel");
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
