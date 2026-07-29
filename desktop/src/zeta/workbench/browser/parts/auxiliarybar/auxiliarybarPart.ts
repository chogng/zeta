import "./auxiliarybarpart.css";
import { type Event } from "../../../../base/common/event.js";
import {
  ViewContainerLocation,
} from "../../../common/views.js";
import type {
  IViewDescriptorService,
} from "../../../services/views/common/viewDescriptorService.js";
import { CompositePart } from "../compositePart.js";
import {
  CompositeBar,
  type CompositeBarSelectionEvent,
} from "../compositebar/compositeBar.js";

/** Construction inputs for the secondary Composite host. */
export interface AuxiliarybarPartOptions {
  readonly ownerDocument: Document;
  readonly viewDescriptorService: IViewDescriptorService;
}

/**
 * Secondary CompositePart for contextual tools such as Chat and inspectors.
 *
 * Feature contributions own the Composites. This Part owns only their
 * location-specific selection and presentation.
 */
export class AuxiliarybarPart extends CompositePart {
  readonly compositeBar: CompositeBar;

  readonly onDidSelectComposite: Event<CompositeBarSelectionEvent>;

  override get minimumWidth(): number { return 180; }
  override get maximumWidth(): number { return 600; }

  constructor(options: AuxiliarybarPartOptions) {
    super("auxiliarybar", options.ownerDocument);
    this.element.setAttribute("aria-label", "Auxiliary sidebar");
    this.compositeBar = this.own(new CompositeBar({
      ownerDocument: options.ownerDocument,
      viewDescriptorService: options.viewDescriptorService,
      location: ViewContainerLocation.AuxiliaryBar,
      ariaLabel: "Secondary side bar views",
    }));
    this.onDidSelectComposite = this.compositeBar.onDidSelectComposite;
    this.titleElement.append(this.compositeBar.element);
  }

  setActiveComposite(compositeId: string): void {
    this.compositeBar.setActiveComposite(compositeId);
  }
}
