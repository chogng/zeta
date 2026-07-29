import "./sidebarpart.css";
import { type Event } from "../../../../base/common/event.js";
import { ViewContainerLocation } from "../../../common/views.js";
import type { IViewDescriptorService } from "../../../services/views/common/viewDescriptorService.js";
import { CompositePart } from "../compositePart.js";
import {
  CompositeBar,
  type CompositeBarSelectionEvent,
} from "../compositebar/compositeBar.js";

/** Construction inputs for a Sidebar Composite host. */
export interface SidebarPartOptions {
  readonly ownerDocument: Document;
  readonly viewDescriptorService: IViewDescriptorService;
  readonly id?: string;
  readonly location?: ViewContainerLocation;
  readonly ariaLabel?: string;
  readonly viewsAriaLabel?: string;
}

/** Reusable CompositePart presented at the side of a host region. */
export class SidebarPart extends CompositePart {
  readonly compositeBar: CompositeBar;

  readonly onDidSelectComposite: Event<CompositeBarSelectionEvent>;

  override get minimumWidth(): number { return 180; }
  override get maximumWidth(): number { return 600; }

  constructor(options: SidebarPartOptions) {
    super(options.id ?? "sidebar", options.ownerDocument);
    this.element.classList.add("zeta-sidebar-part");
    this.element.setAttribute("aria-label", options.ariaLabel ?? "Primary sidebar");
    this.compositeBar = this.own(new CompositeBar({
      ownerDocument: options.ownerDocument,
      viewDescriptorService: options.viewDescriptorService,
      location: options.location ?? ViewContainerLocation.Sidebar,
      ariaLabel: options.viewsAriaLabel ?? "Primary side bar views",
    }));
    this.onDidSelectComposite = this.compositeBar.onDidSelectComposite;
    this.contentElement.before(this.compositeBar.element);
  }

  setActiveComposite(compositeId: string): void {
    this.compositeBar.setActiveComposite(compositeId);
  }
}
