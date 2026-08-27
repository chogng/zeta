import { setHoverDelegate } from "../../base/browser/ui/hover/hoverDelegate.js";
import { Disposable } from "../../base/common/lifecycle.js";
import { IMenuService, MenuService } from "../../platform/actions/common/menuService.js";
import { ICommandService } from "../../platform/commands/common/commands.js";
import { type IConfigurationService as IConfigurationServiceContract } from "../../platform/configuration/common/configurationService.js";
import { IContextKeyService, ContextKeyService } from "../../platform/contextkey/common/contextkey.js";
import { IContextMenuService } from "../../platform/contextview/browser/contextMenu.js";
import { IContextViewService } from "../../platform/contextview/browser/contextView.js";
import { BrowserContextViewService } from "../../platform/contextview/browser/contextViewService.js";
import { HoverService } from "../../platform/hover/browser/hoverService.js";
import { IHoverService } from "../../platform/hover/common/hoverService.js";
import type { ServiceContainer } from "../../platform/instantiation/common/instantiation.js";
import { IKeybindingService } from "../../platform/keybinding/common/keybinding.js";
import { type IKeybindingsResourceApi, IKeybindingsResourceService } from "../../platform/keybinding/common/keybindingsResource.js";
import { IKeyboardLayoutService, type IKeyboardLayoutProvider } from "../../platform/keyboardLayout/common/keyboardLayout.js";
import {
	IUserKeyboardLayoutService,
	type IUserKeyboardLayoutApi,
	UnavailableUserKeyboardLayoutService,
} from "../../platform/keyboardLayout/common/userKeyboardLayout.js";
import { type ILayoutService as ILayoutServiceContract } from "../../platform/layout/common/layoutService.js";
import { IQuickInputService } from "../../platform/quickinput/common/quickInput.js";
import { CommandService } from "../services/commands/common/commandService.js";
import type { WorkbenchContextMenuServiceFactory } from "../services/contextmenu/browser/workbenchContextMenuService.js";
import { BrowserKeyboardLayoutService } from "../services/keybinding/browser/keyboardLayoutService.js";
import { WorkbenchKeybindingService } from "../services/keybinding/browser/keybindingService.js";
import { IKeyboardShortcutTroubleshootingService } from "../services/keybinding/common/keyboardShortcutTroubleshooting.js";
import { WorkbenchKeybindingsResourceService } from "../services/keybinding/browser/keybindingsResourceService.js";
import { IPreferencesService } from "../services/preferences/common/preferences.js";
import { PreferencesService } from "../services/preferences/browser/preferencesService.js";
import { IEditorService } from "../services/editor/common/editorService.js";
import { WorkbenchQuickInputService } from "../services/quickinput/browser/quickInputService.js";
import { ChatContextPickService } from "../services/chat/browser/chatContextPickService.js";
import { IChatContextPickService, type IChatContextPickService as IChatContextPickServiceContract } from "../services/chat/common/chatContextService.js";
import type { IStatusbarService } from "../services/statusbar/browser/statusbar.js";

export interface WorkbenchInteractionServicesOptions {
	readonly container: ServiceContainer;
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
export class WorkbenchInteractionServices extends Disposable {
	readonly commandService: CommandService;
	readonly contextKeyService: ContextKeyService;
	readonly menuService: MenuService;
	readonly contextViewService: BrowserContextViewService;
	readonly contextMenuService: IContextMenuService;
	readonly keybindingService: WorkbenchKeybindingService;
	readonly quickInputService: WorkbenchQuickInputService;
	readonly chatContextPickService: IChatContextPickServiceContract;

	constructor(options: WorkbenchInteractionServicesOptions) {
		super();
		const ownerDocument = options.layoutService.activeContainer.ownerDocument;
		const ownerWindow = ownerDocument.defaultView;
		if (!ownerWindow) throw new Error("Workbench interaction services require an owner window");
		const container = options.container;
		const userKeyboardLayoutService = options.userKeyboardLayoutApi ?? UnavailableUserKeyboardLayoutService;
		container.registerInstance(IUserKeyboardLayoutService, userKeyboardLayoutService);

		this.commandService = this._register(new CommandService(container));
		container.registerInstance(ICommandService, this.commandService);
		this.contextKeyService = this._register(new ContextKeyService());
		container.registerInstance(IContextKeyService, this.contextKeyService);

		const keyboardLayoutService = this._register(new BrowserKeyboardLayoutService({
			navigator: ownerWindow.navigator,
			configurationService: options.configurationService,
			layoutProvider: options.keyboardLayoutProvider,
			userLayoutProvider: userKeyboardLayoutService,
		}));
		container.registerInstance(IKeyboardLayoutService, keyboardLayoutService);
		const keybindingsResourceService = this._register(new WorkbenchKeybindingsResourceService({
			api: options.keybindingsResourceApi,
		}));
		container.registerInstance(IKeybindingsResourceService, keybindingsResourceService);
		this.keybindingService = this._register(new WorkbenchKeybindingService({
			ownerDocument,
			commandService: this.commandService,
			contextKeyService: this.contextKeyService,
			keyboardLayoutService,
			statusbarService: options.statusbarService,
		}));
		container.registerInstance(IKeybindingService, this.keybindingService);
		container.registerInstance(IKeyboardShortcutTroubleshootingService, this.keybindingService);

		this.menuService = new MenuService(this.commandService, this.contextKeyService);
		container.registerInstance(IMenuService, this.menuService);
		this.contextViewService = this._register(new BrowserContextViewService(options.layoutService.activeContainer, options.layoutService));
		container.registerInstance(IContextViewService, this.contextViewService);
		const quickInputService = this._register(new WorkbenchQuickInputService({
			container: options.layoutService.activeContainer,
			contextKeyService: this.contextKeyService,
			layoutService: options.layoutService,
		}));
		this.quickInputService = quickInputService;
		container.registerInstance(IQuickInputService, quickInputService);
		this.chatContextPickService = new ChatContextPickService();
		container.registerInstance(IChatContextPickService, this.chatContextPickService);
		container.registerInstance(IPreferencesService, this._register(new PreferencesService(() => container.get(IEditorService))));
		this.contextMenuService = this._register(options.createContextMenuService({
			menuService: this.menuService,
			keybindingService: this.keybindingService,
			contextViewService: this.contextViewService,
		}));
		container.registerInstance(IContextMenuService, this.contextMenuService);
		const hoverService = this._register(new HoverService(options.configurationService, this.contextViewService, this.contextMenuService));
		container.registerInstance(IHoverService, hoverService);
		this._register(setHoverDelegate(hoverService));

		void options.configurationService.reload().catch((error: unknown) => {
			console.error("Failed to initialize configuration", error);
		});
		void keybindingsResourceService.reload().catch((error: unknown) => {
			console.error("Failed to initialize keybindings resource", error);
		});
	}
}
