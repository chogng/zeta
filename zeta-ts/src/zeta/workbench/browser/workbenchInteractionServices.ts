import { setHoverDelegate } from "../../base/browser/ui/hover/hoverDelegate.js";
import { DisposableOwner } from "../../base/common/lifecycle.js";
import { IMenuService, MenuService } from "../../platform/actions/common/menuService.js";
import { ICommandService } from "../../platform/commands/common/commands.js";
import { IConfigurationService, type IConfigurationService as IConfigurationServiceContract } from "../../platform/configuration/common/configurationService.js";
import { IContextKeyService, ContextKeyService } from "../../platform/contextkey/common/contextkey.js";
import { IContextMenuService } from "../../platform/contextview/browser/contextMenu.js";
import { IContextViewService } from "../../platform/contextview/browser/contextView.js";
import { BrowserContextViewService } from "../../platform/contextview/browser/contextViewService.js";
import { HoverService } from "../../platform/hover/browser/hoverService.js";
import { IHoverService } from "../../platform/hover/common/hoverService.js";
import type { ServiceCollection } from "../../platform/instantiation/common/instantiation.js";
import { IKeybindingService } from "../../platform/keybinding/common/keybinding.js";
import { type IKeybindingsResourceApi, IKeybindingsResourceService } from "../../platform/keybinding/common/keybindingsResource.js";
import { IKeyboardLayoutService, type IKeyboardLayoutProvider } from "../../platform/keyboardLayout/common/keyboardLayout.js";
import {
	IUserKeyboardLayoutService,
	type IUserKeyboardLayoutApi,
	UnavailableUserKeyboardLayoutService,
} from "../../platform/keyboardLayout/common/userKeyboardLayout.js";
import { ILayoutService, type ILayoutService as ILayoutServiceContract } from "../../platform/layout/common/layoutService.js";
import { IQuickInputService } from "../../platform/quickinput/common/quickInput.js";
import { CommandService } from "../services/commands/common/commandService.js";
import type { WorkbenchContextMenuServiceFactory } from "../services/contextmenu/browser/workbenchContextMenuService.js";
import { BrowserKeyboardLayoutService } from "../services/keybinding/browser/keyboardLayoutService.js";
import { WorkbenchKeybindingService } from "../services/keybinding/browser/keybindingService.js";
import { IKeyboardShortcutTroubleshootingService } from "../services/keybinding/common/keyboardShortcutTroubleshooting.js";
import { WorkbenchKeybindingsResourceService } from "../services/keybinding/browser/keybindingsResourceService.js";
import { ISettingsService } from "../services/preferences/common/settings.js";
import { SettingsService } from "../services/preferences/common/settingsService.js";
import { WorkbenchQuickInputService } from "../services/quickinput/browser/quickInputService.js";
import type { IStatusbarService } from "../services/statusbar/browser/statusbar.js";

export interface WorkbenchInteractionServicesOptions {
	readonly services: ServiceCollection;
	readonly layoutService: ILayoutServiceContract;
	readonly configurationService: IConfigurationServiceContract;
	readonly keybindingsResourceApi?: IKeybindingsResourceApi;
	readonly keyboardLayoutProvider?: IKeyboardLayoutProvider;
	readonly userKeyboardLayoutApi?: IUserKeyboardLayoutApi;
	readonly statusbarService?: IStatusbarService;
	readonly createContextMenuService: WorkbenchContextMenuServiceFactory;
}

/**
 * Window-scoped interaction runtime shared by the regular and specialized
 * Workbenches. Product hosts provide layout and configuration ownership while
 * this runtime owns the canonical command, context, menu, keybinding, overlay,
 * quick-input, settings, and hover service graph.
 */
export class WorkbenchInteractionServices extends DisposableOwner {
	readonly commandService: CommandService;
	readonly contextKeyService: ContextKeyService;
	readonly menuService: MenuService;
	readonly contextViewService: BrowserContextViewService;
	readonly contextMenuService: IContextMenuService;
	readonly keybindingService: WorkbenchKeybindingService;

	constructor(options: WorkbenchInteractionServicesOptions) {
		super();
		const ownerDocument = options.layoutService.activeContainer.ownerDocument;
		const ownerWindow = ownerDocument.defaultView;
		if (!ownerWindow) throw new Error("Workbench interaction services require an owner window");
		const services = options.services;
		services.set(ILayoutService, options.layoutService);
		services.set(IConfigurationService, options.configurationService);
		const userKeyboardLayoutService = options.userKeyboardLayoutApi ?? UnavailableUserKeyboardLayoutService;
		services.set(IUserKeyboardLayoutService, userKeyboardLayoutService);

		this.commandService = this.own(new CommandService(services));
		services.set(ICommandService, this.commandService);
		this.contextKeyService = this.own(new ContextKeyService());
		services.set(IContextKeyService, this.contextKeyService);

		const keyboardLayoutService = this.own(new BrowserKeyboardLayoutService({
			navigator: ownerWindow.navigator,
			configurationService: options.configurationService,
			layoutProvider: options.keyboardLayoutProvider,
			userLayoutProvider: userKeyboardLayoutService,
		}));
		services.set(IKeyboardLayoutService, keyboardLayoutService);
		const keybindingsResourceService = this.own(new WorkbenchKeybindingsResourceService({
			api: options.keybindingsResourceApi,
		}));
		services.set(IKeybindingsResourceService, keybindingsResourceService);
		this.keybindingService = this.own(new WorkbenchKeybindingService({
			ownerDocument,
			commandService: this.commandService,
			contextKeyService: this.contextKeyService,
			keyboardLayoutService,
			statusbarService: options.statusbarService,
		}));
		services.set(IKeybindingService, this.keybindingService);
		services.set(IKeyboardShortcutTroubleshootingService, this.keybindingService);

		this.menuService = new MenuService(this.commandService, this.contextKeyService);
		services.set(IMenuService, this.menuService);
		this.contextViewService = this.own(new BrowserContextViewService(options.layoutService.activeContainer, options.layoutService));
		services.set(IContextViewService, this.contextViewService);
		const quickInputService = this.own(new WorkbenchQuickInputService({
			container: options.layoutService.activeContainer,
			contextKeyService: this.contextKeyService,
			layoutService: options.layoutService,
		}));
		services.set(IQuickInputService, quickInputService);
		services.set(ISettingsService, this.own(new SettingsService()));
		this.contextMenuService = this.own(options.createContextMenuService({
			menuService: this.menuService,
			keybindingService: this.keybindingService,
			contextViewService: this.contextViewService,
		}));
		services.set(IContextMenuService, this.contextMenuService);
		const hoverService = this.own(new HoverService(options.configurationService, this.contextViewService, this.contextMenuService));
		services.set(IHoverService, hoverService);
		this.own(setHoverDelegate(hoverService));

		void options.configurationService.reload().catch((error: unknown) => {
			console.error("Failed to initialize configuration", error);
		});
		void keybindingsResourceService.reload().catch((error: unknown) => {
			console.error("Failed to initialize keybindings resource", error);
		});
	}
}
