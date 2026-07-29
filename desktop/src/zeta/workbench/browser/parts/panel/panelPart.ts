import "./panelpart.css";
import { type Event } from "../../../../base/common/event.js";
import { ViewContainerLocation } from "../../../common/views.js";
import type { IViewDescriptorService } from "../../../services/views/common/viewDescriptorService.js";
import { CompositePart } from "../compositePart.js";
import { CompositeBar, type CompositeBarSelectionEvent } from "../compositebar/compositeBar.js";

/** Construction inputs for the bottom Panel Composite host. */
export interface PanelPartOptions {
  readonly ownerDocument: Document;
  readonly viewDescriptorService: IViewDescriptorService;
}

/** Bottom tool region with Panel tabs and a contextual title toolbar. */
export class PanelPart extends CompositePart {
  readonly compositeBar: CompositeBar;
  readonly onDidSelectComposite: Event<CompositeBarSelectionEvent>;
  readonly #actionsElement: HTMLDivElement;

  override get minimumHeight(): number { return 80; }

  constructor(options: PanelPartOptions) {
    super("panel", options.ownerDocument);
    this.element.setAttribute("aria-label", "Panel");
    this.compositeBar = this.own(new CompositeBar({
      ownerDocument: options.ownerDocument,
      viewDescriptorService: options.viewDescriptorService,
      location: ViewContainerLocation.Panel,
      ariaLabel: "Panel views",
    }));
    this.onDidSelectComposite = this.compositeBar.onDidSelectComposite;
    const titleControl = options.ownerDocument.createElement("div");
    titleControl.className = "zeta-panel-title-control";
    this.#actionsElement = options.ownerDocument.createElement("div");
    this.#actionsElement.className = "zeta-panel-title-actions";
    titleControl.append(this.compositeBar.element, this.#actionsElement);
    this.contentElement.before(titleControl);
  }

  setActiveComposite(compositeId: string): void {
    this.compositeBar.setActiveComposite(compositeId);
  }

  override showComposite(compositeId: string): void {
    super.showComposite(compositeId);
    this.#actionsElement.replaceChildren(
      ...optionalElement(this.getComposite(compositeId)?.titleActionsElement),
    );
  }
}

function optionalElement(element: HTMLElement | undefined): HTMLElement[] {
  return element ? [element] : [];
}
