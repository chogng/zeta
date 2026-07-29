import "./sidebarpart.css";
import { type Event } from "../../../../base/common/event.js";
import { ViewContainerLocation } from "../../../common/views.js";
import type { IViewDescriptorService } from "../../../services/views/common/viewDescriptorService.js";
import { CompositePart } from "../compositePart.js";
import {
  CompositeBar,
  type CompositeBarSelectionEvent,
} from "../compositebar/compositeBar.js";

/** Construction inputs for the primary Sidebar Composite host. */
export interface SidebarPartOptions {
  readonly ownerDocument: Document;
  readonly viewDescriptorService: IViewDescriptorService;
}

/** Primary CompositePart presented at the side of the workbench. */
export class SidebarPart extends CompositePart {
  readonly compositeBar: CompositeBar;

  readonly onDidSelectComposite: Event<CompositeBarSelectionEvent>;

  override get minimumWidth(): number { return 180; }
  override get maximumWidth(): number { return 600; }

  constructor(options: SidebarPartOptions) {
    super("sidebar", options.ownerDocument);
    this.element.setAttribute("aria-label", "Primary sidebar");
    this.compositeBar = this.own(new CompositeBar({
      ownerDocument: options.ownerDocument,
      viewDescriptorService: options.viewDescriptorService,
      location: ViewContainerLocation.Sidebar,
      ariaLabel: "Primary side bar views",
    }));
    this.onDidSelectComposite = this.compositeBar.onDidSelectComposite;
    this.contentElement.before(this.compositeBar.element);
  }

  setActiveComposite(compositeId: string): void {
    this.compositeBar.setActiveComposite(compositeId);
  }
}
