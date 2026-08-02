import "./sidebarpart.css";
import { ViewContainerLocation, type IViewContainerDescriptor } from "../../../common/views.js";
import type { IViewDescriptorService } from "../../../services/views/common/viewDescriptorService.js";
import { PaneCompositePart, type PaneCompositeTitleActions } from "../paneCompositePart.js";

/** Construction inputs for a Sidebar Composite host. */
export interface SidebarPartOptions {
  readonly ownerDocument: Document;
  readonly viewDescriptorService: IViewDescriptorService;
  readonly id?: string;
  readonly location?: ViewContainerLocation;
  readonly ariaLabel?: string;
  readonly viewsAriaLabel?: string;
  /** Selects which registered containers receive items in the hosted CompositeBar. */
  readonly compositeBarContainerFilter?: (container: IViewContainerDescriptor) => boolean;
  readonly compositeBarVisible?: boolean;
  readonly titleActions?: PaneCompositeTitleActions;
}

/** Reusable Pane Composite Part presented at the side of the Workbench. */
export class SidebarPart extends PaneCompositePart {
  override get minimumWidth(): number { return 180; }
  override get maximumWidth(): number { return 600; }

  constructor(options: SidebarPartOptions) {
    super({
      ownerDocument: options.ownerDocument,
      viewDescriptorService: options.viewDescriptorService,
      id: options.id ?? "sidebar",
      location: options.location ?? ViewContainerLocation.Sidebar,
      ariaLabel: options.ariaLabel ?? "Primary sidebar",
      viewsAriaLabel: options.viewsAriaLabel ?? "Primary side bar views",
      compositeBarContainerFilter: options.compositeBarContainerFilter,
      compositeBarVisible: options.compositeBarVisible,
      titleActions: options.titleActions,
    });
    this.element.classList.add("zeta-sidebar-part");
  }
}
