import "./panelpart.css";
import type { IContextMenuProvider } from "../../../../base/browser/contextmenu.js";
import type { IStorageService } from "../../../../platform/storage/common/storage.js";
import { ViewContainerLocation } from "../../../common/views.js";
import type { ILocalizationService } from "../../../services/localization/common/localizationService.js";
import type { IViewDescriptorService } from "../../../services/views/common/viewDescriptorService.js";
import { PaneCompositePart, type PaneCompositeTitleActions } from "../paneCompositePart.js";

/** Construction inputs for the bottom Panel Composite host. */
export interface PanelPartOptions {
	readonly viewDescriptorService: IViewDescriptorService;
	readonly storageService?: IStorageService;
	readonly localizationService?: ILocalizationService;
	readonly contextMenuProvider?: IContextMenuProvider;
	readonly titleActions?: PaneCompositeTitleActions;
}

/** Bottom tool region with Panel tabs and a contextual title toolbar. */
export class PanelPart extends PaneCompositePart {
	override get minimumHeight(): number { return 80; }

	constructor(container: HTMLElement, options: PanelPartOptions) {
		super(container, {
			viewDescriptorService: options.viewDescriptorService,
			storageService: options.storageService,
			localizationService: options.localizationService,
			id: "panel",
			location: ViewContainerLocation.Panel,
			ariaLabel: "Panel",
			ariaLabelKey: { bundle: "zeta.regions", key: "panel" },
			viewsAriaLabel: "Panel views",
			viewsAriaLabelKey: { bundle: "zeta.regions", key: "panelViews" },
			compositeBarPresentation: "label",
			compositeBarContextMenuProvider: options.contextMenuProvider,
			titleActions: options.titleActions,
		});
		this.titleDomNode.classList.add("zeta-panel-title-control");
		this.titleActionsSlotDomNode.classList.add("zeta-panel-title-actions");
	}

	override showComposite(compositeId: string): void {
		super.showComposite(compositeId);
		this.setTitleProjection(this.getComposite(compositeId)?.partTitleProjection);
	}
}
