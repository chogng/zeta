import "./paneCompositePart.css";
import type { IContextMenuProvider } from "../../../base/browser/contextmenu.js";
import type { IDimension } from "../../../base/browser/geometry.js";
import { type Event } from "../../../base/common/event.js";
import { MenuWorkbenchToolBar } from "../../../platform/actions/browser/toolbar.js";
import { type MenuId } from "../../../platform/actions/common/actions.js";
import type { IMenuService } from "../../../platform/actions/common/menuService.js";
import { ViewContainerLocation, type IViewContainerDescriptor } from "../../common/views.js";
import type { IViewDescriptorService } from "../../services/views/common/viewDescriptorService.js";
import { CompositePart } from "./compositePart.js";
import { CompositeBar, type CompositeBarPresentation, type CompositeBarSelectionEvent } from "./compositebar/compositeBar.js";

/** Menu-backed actions rendered at the right edge of a Pane Composite title. */
export interface PaneCompositeTitleActions {
  readonly menuService: IMenuService;
  readonly contextMenuProvider: IContextMenuProvider;
  readonly menuId: MenuId;
}

/** Construction inputs shared by Sidebars, Auxiliary Bar, and Panel. */
export interface PaneCompositePartOptions {
  readonly ownerDocument: Document;
  readonly viewDescriptorService: IViewDescriptorService;
  readonly id: string;
  readonly location: ViewContainerLocation;
  readonly ariaLabel: string;
  readonly viewsAriaLabel: string;
  readonly compositeBarPresentation?: CompositeBarPresentation;
  readonly compositeBarContextMenuProvider?: IContextMenuProvider;
  /** Selects which registered containers receive items in the hosted CompositeBar. */
  readonly compositeBarContainerFilter?: (container: IViewContainerDescriptor) => boolean;
  readonly compositeBarVisible?: boolean;
  readonly titleActions?: PaneCompositeTitleActions;
}

/**
 * Standard Workbench host for a location's retained PaneComposites.
 *
 * It owns the title slot, CompositeBar, and Composite lifecycle. Concrete
 * Parts only supply region constraints and
 * location-specific presentation.
 */
export class PaneCompositePart extends CompositePart {
  readonly compositeBar: CompositeBar;
  readonly onDidSelectComposite: Event<CompositeBarSelectionEvent>;
  private readonly titleContentElement: HTMLDivElement;
  protected readonly titleActionsSlotElement: HTMLDivElement;
  private compositeBarVisible = true;
  private hasCustomTitleContent = false;

  constructor(options: PaneCompositePartOptions) {
    super(options.id, options.ownerDocument);
    this.element.setAttribute("aria-label", options.ariaLabel);
    this.titleElement.classList.add("zeta-pane-composite-title");
    this.compositeBar = this.own(new CompositeBar({
      ownerDocument: options.ownerDocument,
      viewDescriptorService: options.viewDescriptorService,
      location: options.location,
      ariaLabel: options.viewsAriaLabel,
      presentation: options.compositeBarPresentation,
      contextMenuProvider: options.compositeBarContextMenuProvider,
      containerFilter: options.compositeBarContainerFilter,
    }));
    this.onDidSelectComposite = this.compositeBar.onDidSelectComposite;
    this.titleContentElement = options.ownerDocument.createElement("div");
    this.titleContentElement.className = "zeta-pane-composite-title-content";
    this.titleContentElement.append(this.compositeBar.element);
    this.titleActionsSlotElement = options.ownerDocument.createElement("div");
    this.titleActionsSlotElement.className = "zeta-pane-composite-title-actions";
    this.titleElement.append(this.titleContentElement, this.titleActionsSlotElement);

    if (options.titleActions) {
      const actions = this.own(new MenuWorkbenchToolBar(
        options.titleActions.menuService,
        options.titleActions.contextMenuProvider,
        options.titleActions.menuId,
        options.ownerDocument,
        { highlightToggledItems: true },
      ));
      actions.element.classList.add("zeta-pane-composite-title-menu-actions");
      this.titleActionsSlotElement.append(actions.element);
    }

    this.setCompositeBarVisible(options.compositeBarVisible ?? true);
  }

  setActiveComposite(compositeId: string): void {
    this.compositeBar.setActiveComposite(compositeId);
  }

  setCompositeBarVisible(visible: boolean): void {
    this.compositeBarVisible = visible;
    this.compositeBar.element.hidden = !visible;
    this.updateTitleVisibility();
  }

  /** Projects view-owned title content into the left title slot. */
  protected setTitleContent(element: HTMLElement | undefined): void {
    this.hasCustomTitleContent = element !== undefined;
    this.titleContentElement.replaceChildren(
      ...(element ? [element] : [this.compositeBar.element]),
    );
    this.updateTitleVisibility();
  }

  /** Projects view-owned actions into the right title slot. */
  protected setTitleActions(element: HTMLElement | undefined): void {
    this.titleActionsSlotElement.replaceChildren(...(element ? [element] : []));
    this.updateTitleVisibility();
  }

  override layout(_dimension: IDimension): void {
    this.compositeBar.layout();
  }

  private updateTitleVisibility(): void {
    this.titleElement.hidden = !this.compositeBarVisible && !this.hasCustomTitleContent && this.titleActionsSlotElement.childElementCount === 0;
  }
}
