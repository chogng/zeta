import "./auxiliarybarpart.css";
import { ViewContainerLocation } from "../../../common/views.js";
import type { IViewDescriptorService } from "../../../services/views/common/viewDescriptorService.js";
import { PaneCompositePart } from "../paneCompositePart.js";

/** Construction inputs for the fixed Auxiliary Bar Pane Composite host. */
export interface AuxiliarybarPartOptions {
  readonly ownerDocument: Document;
  readonly viewDescriptorService: IViewDescriptorService;
}

/**
 * Secondary Pane Composite region.
 *
 * Its fixed Chat container still owns session navigation, while this Part
 * owns the retained Composite lifecycle shared by all pane-like regions.
 */
export class AuxiliarybarPart extends PaneCompositePart {

  override get minimumWidth(): number { return 180; }
  override get maximumWidth(): number { return 600; }

  constructor(options: AuxiliarybarPartOptions) {
    super({
      ownerDocument: options.ownerDocument,
      viewDescriptorService: options.viewDescriptorService,
      id: "auxiliarybar",
      location: ViewContainerLocation.AuxiliaryBar,
      ariaLabel: "Auxiliary sidebar",
      viewsAriaLabel: "Auxiliary sidebar views",
      compositeBarVisible: false,
    });
    this.contentElement.classList.add("zeta-auxiliarybar-content");
  }

  override showComposite(compositeId: string): void {
    super.showComposite(compositeId);
    const composite = this.getComposite(compositeId);
    this.setTitleContent(composite?.partTitleElement);
    this.setTitleActions(composite?.partTitleActionsElement);
  }
}
