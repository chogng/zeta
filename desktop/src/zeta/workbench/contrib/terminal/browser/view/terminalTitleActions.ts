import { ActionViewItem, ButtonActionViewItem, LabelActionViewItem } from "../../../../../base/browser/ui/actionbar/actionViewItems.js";
import { DropdownMenuActionViewItem } from "../../../../../base/browser/ui/dropdown/dropdownMenuActionViewItem.js";
import type { IAction } from "../../../../../base/common/actions.js";
import { lxiconsLibrary } from "../../../../../base/common/lxiconsLibrary.js";
import { DisposableOwner } from "../../../../../base/common/lifecycle.js";
import { MenuWorkbenchToolBar } from "../../../../../platform/actions/browser/toolbar.js";
import { MenuId, MenusRegistry } from "../../../../../platform/actions/common/actions.js";
import type { IMenuService } from "../../../../../platform/actions/common/menuService.js";
import { CommandsRegistry } from "../../../../../platform/commands/common/commands.js";
import { ContextKeyExpr, type IContextKey, type IContextKeyService, RawContextKey } from "../../../../../platform/contextkey/common/contextkey.js";
import type { IContextMenuService } from "../../../../../platform/contextview/browser/contextMenu.js";
import { ToggleMaximizedPanelCommandId, TogglePanelCommandId } from "../../../../browser/parts/titlebar/titlebarActions.js";
import type { ITerminalInstance, ITerminalProfile } from "../../../../services/terminal/common/terminal.js";
import { terminalProfileIcon } from "./terminalProfileIcon.js";

const ACTIVE_TERMINAL_COMMAND_ID = "zeta.terminal.focusActive";
const NEW_TERMINAL_WITH_PROFILE_COMMAND_ID = "zeta.terminal.newWithProfile";
const NEW_TERMINAL_COMMAND_ID = "zeta.terminal.new";
const RELAUNCH_TERMINAL_COMMAND_ID = "zeta.terminal.relaunch";
const KILL_TERMINAL_COMMAND_ID = "zeta.terminal.kill";
const CLEAR_TERMINAL_COMMAND_ID = "zeta.terminal.clear";
const TerminalProfilesAvailableContext = new RawContextKey<boolean>("terminalProfilesAvailable", false);
const TerminalCreatingContext = new RawContextKey<boolean>("terminalCreating", false);
const TerminalHasActiveInstanceContext = new RawContextKey<boolean>("terminalHasActiveInstance", false);
const TerminalActiveInstanceInTitleContext = new RawContextKey<boolean>("terminalActiveInstanceInTitle", false);
const TerminalActiveInstanceStateContext = new RawContextKey<ITerminalInstance["state"] | "none">("terminalActiveInstanceState", "none");

export interface TerminalTitleActionsOptions {
  readonly ownerDocument: Document;
  readonly menuService: IMenuService;
  readonly contextMenuService: IContextMenuService;
  readonly contextKeyService: IContextKeyService;
  readonly createTerminal: (profileId?: string) => unknown;
  readonly focusActive: () => unknown;
  readonly relaunchActive: () => unknown;
  readonly killActive: () => unknown;
  readonly clearActive: () => unknown;
}

/** Owns the Terminal title Command, Menu, Context Key, and Toolbar projection. */
export class TerminalTitleActions extends DisposableOwner {
  readonly element: HTMLElement;
  private readonly toolbar: MenuWorkbenchToolBar;
  private readonly profilesAvailableContext: IContextKey<boolean>;
  private readonly creatingContext: IContextKey<boolean>;
  private readonly hasActiveInstanceContext: IContextKey<boolean>;
  private readonly activeInstanceInTitleContext: IContextKey<boolean>;
  private readonly activeInstanceStateContext: IContextKey<ITerminalInstance["state"] | "none">;
  private profiles: readonly ITerminalProfile[] = [];
  private activeInstance: ITerminalInstance | undefined;

  constructor(options: TerminalTitleActionsOptions) {
    super();
    this.profilesAvailableContext = TerminalProfilesAvailableContext.bindTo(options.contextKeyService);
    this.creatingContext = TerminalCreatingContext.bindTo(options.contextKeyService);
    this.hasActiveInstanceContext = TerminalHasActiveInstanceContext.bindTo(options.contextKeyService);
    this.activeInstanceInTitleContext = TerminalActiveInstanceInTitleContext.bindTo(options.contextKeyService);
    this.activeInstanceStateContext = TerminalActiveInstanceStateContext.bindTo(options.contextKeyService);
    this.defer(() => {
      this.activeInstanceStateContext.reset();
      this.activeInstanceInTitleContext.reset();
      this.hasActiveInstanceContext.reset();
      this.creatingContext.reset();
      this.profilesAvailableContext.reset();
    });
    this.registerCommandsAndMenu(options);
    this.toolbar = this.own(new MenuWorkbenchToolBar(
      options.menuService,
      options.contextMenuService,
      MenuId.TerminalTitle,
      options.ownerDocument,
      {
        ariaLabel: "Terminal actions",
        highlightToggledItems: true,
        moreActionsPlacement: {
          beforeActionId: ToggleMaximizedPanelCommandId,
        },
        menuOptions: { shouldForwardArgs: true },
        actionViewItemProvider: (action) => this.createActionViewItem(action, options.contextMenuService),
      },
    ));
    this.element = this.toolbar.element;
    this.element.classList.add("zeta-terminal-title-toolbar");
  }

  setProfiles(profiles: readonly ITerminalProfile[]): void {
    this.profiles = profiles;
    this.profilesAvailableContext.set(profiles.length > 0);
    this.toolbar.refresh();
  }

  setCreating(creating: boolean): void {
    this.creatingContext.set(creating);
  }

  setSupplementalSecondaryActions(actions: readonly IAction[]): void {
    this.toolbar.setSupplementalSecondaryActions(actions);
  }

  setActiveInstance(instance: ITerminalInstance | undefined, placement: "list" | "title"): void {
    this.activeInstance = instance;
    this.activeInstanceInTitleContext.set(instance !== undefined && placement === "title");
    this.hasActiveInstanceContext.set(instance !== undefined);
    this.activeInstanceStateContext.set(instance?.state ?? "none");
    this.toolbar.refresh();
  }

  private registerCommandsAndMenu(options: TerminalTitleActionsOptions): void {
    this.own(CommandsRegistry.register(ACTIVE_TERMINAL_COMMAND_ID, () => options.focusActive()));
    this.own(CommandsRegistry.register(NEW_TERMINAL_WITH_PROFILE_COMMAND_ID, (_accessor, profileId) => {
      if (typeof profileId !== "string" || !this.profiles.some((profile) => profile.profileId === profileId)) {
        throw new TypeError(`Unknown terminal profile: ${String(profileId)}`);
      }
      return options.createTerminal(profileId);
    }));
    this.own(CommandsRegistry.register(NEW_TERMINAL_COMMAND_ID, () => options.createTerminal()));
    this.own(CommandsRegistry.register(RELAUNCH_TERMINAL_COMMAND_ID, () => options.relaunchActive()));
    this.own(CommandsRegistry.register(KILL_TERMINAL_COMMAND_ID, () => options.killActive()));
    this.own(CommandsRegistry.register(CLEAR_TERMINAL_COMMAND_ID, () => options.clearActive()));
    this.own(MenusRegistry.appendMenuItem(MenuId.TerminalTitle, {
      command: {
        id: ACTIVE_TERMINAL_COMMAND_ID,
        title: "Focus Active Terminal",
        tooltip: "Focus Active Terminal",
      },
      when: TerminalActiveInstanceInTitleContext.isEqualTo(true),
      group: "navigation",
      order: 0,
    }));
    this.own(MenusRegistry.appendMenuItem(MenuId.TerminalTitle, {
      command: {
        id: NEW_TERMINAL_WITH_PROFILE_COMMAND_ID,
        title: "New Terminal With Profile",
        tooltip: "Select Terminal Profile",
      },
      when: TerminalProfilesAvailableContext.isEqualTo(true),
      group: "navigation",
      order: 11,
    }));
    this.own(MenusRegistry.appendMenuItem(MenuId.TerminalTitle, {
      command: {
        id: NEW_TERMINAL_COMMAND_ID,
        title: "New Terminal",
        tooltip: "New Terminal",
        icon: lxiconsLibrary.add,
        precondition: TerminalCreatingContext.isEqualTo(false),
      },
      group: "navigation",
      order: 10,
    }));
    this.own(MenusRegistry.appendMenuItem(MenuId.TerminalTitle, {
      command: {
        id: RELAUNCH_TERMINAL_COMMAND_ID,
        title: "Relaunch Terminal",
        tooltip: "Relaunch Terminal",
        icon: lxiconsLibrary.history,
      },
      when: ContextKeyExpr.and(
        TerminalHasActiveInstanceContext.isEqualTo(true),
        ContextKeyExpr.notEquals(TerminalActiveInstanceStateContext.key, "running"),
      ),
      group: "navigation",
      order: 20,
    }));
    this.own(MenusRegistry.appendMenuItem(MenuId.TerminalTitle, {
      command: {
        id: KILL_TERMINAL_COMMAND_ID,
        title: "Kill Terminal",
        tooltip: "Kill Terminal",
        icon: lxiconsLibrary.trash,
      },
      when: TerminalHasActiveInstanceContext.isEqualTo(true),
      group: "navigation",
      order: 30,
    }));
    this.own(MenusRegistry.appendMenuItem(MenuId.TerminalTitle, {
      command: {
        id: CLEAR_TERMINAL_COMMAND_ID,
        title: "Clear Terminal",
        tooltip: "Clear Terminal",
      },
      group: "1_terminal",
      order: 10,
    }));
    this.own(MenusRegistry.appendMenuItem(MenuId.TerminalTitle, {
      command: {
        id: TogglePanelCommandId,
        title: "Close Panel",
        tooltip: "Close Panel",
        icon: lxiconsLibrary.close,
      },
      group: "navigation",
      order: 50,
    }));
  }

  private createActionViewItem(action: IAction, contextMenuService: IContextMenuService): ActionViewItem | undefined {
    switch (action.id) {
      case ACTIVE_TERMINAL_COMMAND_ID:
        if (!this.activeInstance) return undefined;
        return new ActiveTerminalActionViewItem(action, this.activeInstance);
      case NEW_TERMINAL_COMMAND_ID:
        return new NewTerminalActionViewItem(action);
      case NEW_TERMINAL_WITH_PROFILE_COMMAND_ID:
        return new TerminalProfileActionViewItem(action, () => this.profiles, () => this.activeInstance?.profile.profileId, contextMenuService);
      default:
        return undefined;
    }
  }
}

class ActiveTerminalActionViewItem extends LabelActionViewItem {
  constructor(action: IAction, private readonly instance: ITerminalInstance) {
    const tooltip = instance.title === instance.profile.title
      ? `Active terminal: ${instance.title}`
      : `Active terminal: ${instance.title} (${instance.profile.title})`;
    super(action, {
      label: instance.title,
      icon: terminalProfileIcon(instance.profile),
      ariaLabel: tooltip,
      tooltip,
    });
  }

  override render(container: HTMLElement): void {
    super.render(container);
    container.classList.add("zeta-terminal-active-action");
    container.dataset.state = this.instance.state;
  }
}

class NewTerminalActionViewItem extends ButtonActionViewItem {
  override render(container: HTMLElement): void {
    super.render(container);
    container.classList.add("zeta-terminal-new-action");
  }
}

class TerminalProfileActionViewItem extends DropdownMenuActionViewItem {
  constructor(action: IAction, profiles: () => readonly ITerminalProfile[], activeProfileId: () => string | undefined, contextMenuService: IContextMenuService) {
    super(
      new TerminalProfileSelectorAction(action),
      () => profiles().map((profile) => terminalProfileMenuAction(action, profile, activeProfileId)),
      contextMenuService,
    );
  }

  override render(container: HTMLElement): void {
    super.render(container);
    container.classList.add("zeta-terminal-profile-action");
    container.querySelector("button")?.setAttribute("aria-label", this.action.tooltip);
  }
}

class TerminalProfileSelectorAction implements IAction {
  constructor(private readonly action: IAction) {}

  get id(): string { return this.action.id; }
  get label(): string { return "Select Terminal Profile"; }
  get tooltip(): string { return this.action.tooltip; }
  get enabled(): boolean { return this.action.enabled; }
  get checked(): boolean | undefined { return this.action.checked; }

  run(...args: readonly unknown[]): unknown {
    return this.action.run(...args);
  }
}

function terminalProfileMenuAction(action: IAction, profile: ITerminalProfile, activeProfileId: () => string | undefined): IAction {
  const label = profile.isDefault ? `${profile.title} (Default)` : profile.title;
  return {
    id: `${action.id}.${profile.profileId}`,
    label,
    tooltip: `Use ${profile.title}`,
    icon: terminalProfileIcon(profile),
    enabled: action.enabled,
    checked: profile.profileId === activeProfileId(),
    run: () => action.run(profile.profileId),
  };
}
