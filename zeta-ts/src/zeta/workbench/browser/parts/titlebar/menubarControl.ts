import "./menubarControl.css";
import { addDisposableListener, h } from "../../../../base/browser/dom.js";
import { Button } from "../../../../base/browser/ui/button/button.js";
import { SubmenuAction } from "../../../../base/common/actions.js";
import { DisposableOwner, type IDisposable } from "../../../../base/common/lifecycle.js";
import { lxiconsLibrary } from "../../../../base/common/lxiconsLibrary.js";
import { MenuId } from "../../../../platform/actions/common/actions.js";
import type { IMenu, IMenuService } from "../../../../platform/actions/common/menuService.js";
import type { IContextMenuService } from "../../../../platform/contextview/browser/contextMenu.js";
import { localize, type ILocalizationService } from "../../../services/localization/common/localizationService.js";

/** Host-selected menubar presentation owned by the titlebar. */
export interface IMenubarControl extends IDisposable {
	readonly element: HTMLElement | undefined;
}

/** Compact application-menu trigger used by web, Windows, and Linux. */
export class BrowserMenubarControl extends DisposableOwner
	implements IMenubarControl {
	private readonly menu: IMenu & Disposable;
	private readonly contextMenuService: IContextMenuService;
	private readonly button: Button;
	private active = false;

	readonly element: HTMLElement;

	constructor(
		container: HTMLElement,
		menuService: IMenuService,
		contextMenuService: IContextMenuService,
		localizationService?: ILocalizationService,
	) {
		super();
		const ownerDocument = container.ownerDocument;
		this.contextMenuService = contextMenuService;
		this.element = h(ownerDocument, "nav");
		this.element.className = "zeta-menubar";
		const applicationMenuLabel = () => localize(localizationService, { bundle: "zeta.regions", key: "applicationMenu" }, "Application menu");
		this.element.setAttribute("aria-label", applicationMenuLabel());
		container.append(this.element);
		this.defer(() => this.element.remove());

		this.menu = this.own(menuService.createMenu(MenuId.MenubarMainMenu));
		this.button = this.own(new Button(this.element, {
			label: applicationMenuLabel(),
			title: applicationMenuLabel(),
			icon: lxiconsLibrary.menu,
			onClick: () => this.toggleMenu(),
		}));
		this.button.domNode.setAttribute("aria-label", applicationMenuLabel());
		if (localizationService) this.own(localizationService.onDidChange(() => {
			const label = applicationMenuLabel();
			this.element.setAttribute("aria-label", label);
			this.button.domNode.setAttribute("aria-label", label);
			this.button.label = label;
			this.button.setTitle(label);
		}));
		this.button.toggleClassName("zeta-menubar-item", true);
		this.button.domNode.setAttribute("aria-haspopup", "menu");
		this.button.domNode.setAttribute("aria-expanded", "false");
		this.own(this.menu.onDidChange(() => {
			if (this.active) this.contextMenuService.hideContextMenu();
		}));
		this.own(addDisposableListener(
			this.button.domNode,
			"keydown",
			(event: KeyboardEvent) => {
				if (
					event.isComposing ||
					event.altKey ||
					event.ctrlKey ||
					event.metaKey
				) {
					return;
				}
				if (event.key === "ArrowDown" || event.key === "Enter") {
					if (!this.active) this.showMenu();
				} else if (event.key === "Escape" && this.active) {
					this.contextMenuService.hideContextMenu();
				} else {
					return;
				}
				event.preventDefault();
				event.stopPropagation();
			},
		));
		this.defer(() => {
			if (this.active) this.contextMenuService.hideContextMenu();
		});
	}

	private toggleMenu(): void {
		if (this.active) {
			this.contextMenuService.hideContextMenu();
			return;
		}
		this.showMenu();
	}

	private showMenu(): void {
		const actions = this.menu.getActions({
			preserveEmptySubmenus: true,
		})
			.flatMap(([, groupActions]) => groupActions)
			.filter((action): action is SubmenuAction =>
				action instanceof SubmenuAction
			);
		if (actions.length === 0) return;

		this.active = true;
		this.button.toggleClassName("active", true);
		this.button.domNode.setAttribute("aria-expanded", "true");
		this.contextMenuService.showContextMenu({
			anchor: this.button.domNode,
			actions,
			onHide: () => {
				this.active = false;
				this.button.toggleClassName("active", false);
				this.button.domNode.setAttribute("aria-expanded", "false");
			},
		});
	}
}
