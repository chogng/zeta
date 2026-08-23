import "./paneCompositePart.css";
import type { IContextMenuProvider } from "../../../base/browser/contextmenu.js";
import type { IDimension } from "../../../base/browser/geometry.js";
import { type Event } from "../../../base/common/event.js";
import { localize, type ILocalizationService, type LocalizationKey } from "../../services/localization/common/localizationService.js";
import { MenuWorkbenchToolBar } from "../../../platform/actions/browser/toolbar.js";
import { type MenuId } from "../../../platform/actions/common/actions.js";
import type { IMenuService } from "../../../platform/actions/common/menuService.js";
import { ViewContainerLocation, type IViewContainerDescriptor } from "../../common/views.js";
import type { IViewDescriptorService } from "../../services/views/common/viewDescriptorService.js";
import { CompositePart } from "./compositePart.js";
import { CompositeBar, type CompositeBarPresentation, type CompositeBarSelectionEvent } from "./compositebar/compositeBar.js";
import type { PartTitleProjection } from "./views/viewPane.js";
import { h } from "../../../base/browser/dom.js";

/** Menu-backed actions rendered at the right edge of a Pane Composite title. */
export interface PaneCompositeTitleActions {
	readonly menuService: IMenuService;
	readonly contextMenuProvider: IContextMenuProvider;
	readonly menuId: MenuId;
}

/** Construction inputs shared by Sidebars, Auxiliary Bar, and Panel. */
export interface PaneCompositePartOptions {
	readonly viewDescriptorService: IViewDescriptorService;
	readonly localizationService?: ILocalizationService;
	readonly id: string;
	readonly location: ViewContainerLocation;
	readonly ariaLabel: string;
	readonly ariaLabelKey?: LocalizationKey;
	readonly viewsAriaLabel: string;
	readonly viewsAriaLabelKey?: LocalizationKey;
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
	private readonly viewTitleActionsElement: HTMLDivElement;
	private readonly partTitleActionsElement: HTMLDivElement;
	private compositeBarVisible = true;
	private hasCustomTitleContent = false;

	constructor(container: HTMLElement, options: PaneCompositePartOptions) {
		super(container, options.id);
		const ownerDocument = container.ownerDocument;
		const ariaLabel = localize(options.localizationService, options.ariaLabelKey, options.ariaLabel);
		const viewsAriaLabel = localize(options.localizationService, options.viewsAriaLabelKey, options.viewsAriaLabel);
		this.element.setAttribute("aria-label", ariaLabel);
		this.titleElement.classList.add("zeta-pane-composite-title");
		this.titleContentElement = h(ownerDocument, "div");
		this.titleContentElement.className = "zeta-pane-composite-title-content";
		this.compositeBar = this.own(new CompositeBar(this.titleContentElement, {
			viewDescriptorService: options.viewDescriptorService,
			localizationService: options.localizationService,
			location: options.location,
			ariaLabel: viewsAriaLabel,
			presentation: options.compositeBarPresentation,
			contextMenuProvider: options.compositeBarContextMenuProvider,
			containerFilter: options.compositeBarContainerFilter,
		}));
		if (options.localizationService) this.own(options.localizationService.onDidChange(() => {
			this.element.setAttribute("aria-label", localize(options.localizationService, options.ariaLabelKey, options.ariaLabel));
			this.compositeBar.setAriaLabel(localize(options.localizationService, options.viewsAriaLabelKey, options.viewsAriaLabel));
		}));
		this.onDidSelectComposite = this.compositeBar.onDidSelectComposite;
		this.titleActionsSlotElement = h(ownerDocument, "div");
		this.titleActionsSlotElement.className = "zeta-pane-composite-title-actions";
		this.viewTitleActionsElement = h(ownerDocument, "div");
		this.viewTitleActionsElement.className = "zeta-pane-composite-title-view-actions";
		this.partTitleActionsElement = h(ownerDocument, "div");
		this.partTitleActionsElement.className = "zeta-pane-composite-title-part-actions";
		this.titleActionsSlotElement.append(this.viewTitleActionsElement, this.partTitleActionsElement);
		this.titleElement.append(this.titleContentElement, this.titleActionsSlotElement);

		if (options.titleActions) {
			const actions = this.own(new MenuWorkbenchToolBar(
				this.partTitleActionsElement,
				options.titleActions.menuService,
				options.titleActions.contextMenuProvider,
				options.titleActions.menuId,
				{ highlightToggledItems: true },
			));
			actions.element.classList.add("zeta-pane-composite-title-menu-actions");
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

	/** Projects one View's title content and actions into the Part's fixed slots. */
	protected setTitleProjection(projection: PartTitleProjection | undefined): void {
		this.hasCustomTitleContent = projection?.content !== undefined;
		this.titleContentElement.replaceChildren(
			...(projection?.content ? [projection.content] : [this.compositeBar.element]),
		);
		this.viewTitleActionsElement.replaceChildren(...(projection?.actions ? [projection.actions] : []));
		this.updateTitleVisibility();
	}

	override layout(_dimension: IDimension): void {
		this.compositeBar.layout();
	}

	private updateTitleVisibility(): void {
		this.titleElement.hidden = !this.compositeBarVisible && !this.hasCustomTitleContent && this.viewTitleActionsElement.childElementCount === 0 && this.partTitleActionsElement.childElementCount === 0;
	}
}
