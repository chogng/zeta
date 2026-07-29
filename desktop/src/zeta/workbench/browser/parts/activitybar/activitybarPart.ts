import "./activitybarpart.css";
import { type Event } from "../../../../base/common/event.js";
import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import {
  ViewContainerLocation,
} from "../../../common/views.js";
import type {
  IViewDescriptorService,
} from "../../../services/views/common/viewDescriptorService.js";
import {
  CompositeBar,
  type CompositeBarSelectionEvent,
} from "../compositebar/compositeBar.js";

/** Construction inputs for the primary Activity Bar host. */
export interface ActivitybarPartOptions {
  readonly ownerDocument: Document;
  readonly viewDescriptorService: IViewDescriptorService;
}

/**
 * Workbench host and controller for the primary CompositeBar.
 *
 * The Activity Bar retains ownership when its CompositeBar is reparented into
 * another presentation slot, such as a Sidebar title or bottom area.
 */
export class ActivitybarPart extends DisposableOwner {
  readonly element: HTMLElement;
  readonly compositeBar: CompositeBar;

  readonly onDidSelectComposite: Event<CompositeBarSelectionEvent>;

  constructor(options: ActivitybarPartOptions) {
    super();
    this.element = options.ownerDocument.createElement("section");
    this.element.className = "zeta-activitybar-container";
    this.element.setAttribute("aria-label", "Activity Bar");
    this.defer(() => this.element.remove());
    this.compositeBar = this.own(new CompositeBar({
      ownerDocument: options.ownerDocument,
      viewDescriptorService: options.viewDescriptorService,
      location: ViewContainerLocation.Sidebar,
      ariaLabel: "Primary side bar views",
    }));
    this.onDidSelectComposite = this.compositeBar.onDidSelectComposite;
    this.placeCompositeBar(this.element);
  }

  get activeCompositeId(): string | undefined {
    return this.compositeBar.activeCompositeId;
  }

  setActiveComposite(compositeId: string): void {
    this.compositeBar.setActiveComposite(compositeId);
  }

  /**
   * Reparents the owned CompositeBar into a location-specific presentation
   * slot without recreating its actions or losing selection state.
   */
  placeCompositeBar(target: Element): void {
    if (target.ownerDocument !== this.element.ownerDocument) {
      throw new Error("Composite Bar target belongs to another document");
    }
    target.append(this.compositeBar.element);
  }

  setVisible(visible: boolean): void {
    this.element.hidden = !visible;
  }
}
