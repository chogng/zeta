import { addDisposableListener } from "../../../../../base/browser/dom.js";
import { ActionViewItem } from "../../../../../base/browser/ui/actionbar/actionViewItems.js";
import type { IAction } from "../../../../../base/common/actions.js";
import { lxiconsLibrary } from "../../../../../base/common/lxiconsLibrary.js";
import { DisposableOwner } from "../../../../../base/common/lifecycle.js";
import { MenuWorkbenchToolBar } from "../../../../../platform/actions/browser/toolbar.js";
import { MenuId, MenusRegistry } from "../../../../../platform/actions/common/actions.js";
import type { IMenuService } from "../../../../../platform/actions/common/menuService.js";
import { CommandsRegistry } from "../../../../../platform/commands/common/commands.js";
import { ContextKeyExpr, type IContextKey, type IContextKeyService, RawContextKey } from "../../../../../platform/contextkey/common/contextkey.js";
import type { IContextMenuService } from "../../../../../platform/contextview/browser/contextMenu.js";
import type { ITerminalInstance, ITerminalProfile } from "../../../../services/terminal/common/terminal.js";

const SELECT_PROFILE_COMMAND_ID = "zeta.terminal.selectProfile";
const NEW_TERMINAL_COMMAND_ID = "zeta.terminal.new";
const RELAUNCH_TERMINAL_COMMAND_ID = "zeta.terminal.relaunch";
const KILL_TERMINAL_COMMAND_ID = "zeta.terminal.kill";
const TerminalProfilesAvailableContext = new RawContextKey<boolean>("terminalProfilesAvailable", false);
const TerminalCreatingContext = new RawContextKey<boolean>("terminalCreating", false);
const TerminalHasActiveInstanceContext = new RawContextKey<boolean>("terminalHasActiveInstance", false);
const TerminalActiveInstanceStateContext = new RawContextKey<ITerminalInstance["state"] | "none">("terminalActiveInstanceState", "none");

export interface TerminalTitleActionsOptions {
  readonly ownerDocument: Document;
  readonly menuService: IMenuService;
  readonly contextMenuService: IContextMenuService;
  readonly contextKeyService: IContextKeyService;
  readonly createTerminal: () => unknown;
  readonly relaunchActive: () => unknown;
  readonly killActive: () => unknown;
}

/** Owns the Terminal title Command, Menu, Context Key, and Toolbar projection. */
export class TerminalTitleActions extends DisposableOwner {
  readonly element: HTMLElement;
  readonly #toolbar: MenuWorkbenchToolBar;
  readonly #profilesAvailableContext: IContextKey<boolean>;
  readonly #creatingContext: IContextKey<boolean>;
  readonly #hasActiveInstanceContext: IContextKey<boolean>;
  readonly #activeInstanceStateContext: IContextKey<ITerminalInstance["state"] | "none">;
  #profiles: readonly ITerminalProfile[] = [];
  #selectedProfileId: string | undefined;

  constructor(options: TerminalTitleActionsOptions) {
    super();
    this.#profilesAvailableContext = TerminalProfilesAvailableContext.bindTo(options.contextKeyService);
    this.#creatingContext = TerminalCreatingContext.bindTo(options.contextKeyService);
    this.#hasActiveInstanceContext = TerminalHasActiveInstanceContext.bindTo(options.contextKeyService);
    this.#activeInstanceStateContext = TerminalActiveInstanceStateContext.bindTo(options.contextKeyService);
    this.defer(() => {
      this.#activeInstanceStateContext.reset();
      this.#hasActiveInstanceContext.reset();
      this.#creatingContext.reset();
      this.#profilesAvailableContext.reset();
    });
    this.#registerCommandsAndMenu(options);
    this.#toolbar = this.own(new MenuWorkbenchToolBar(
      options.menuService,
      options.contextMenuService,
      MenuId.TerminalTitle,
      options.ownerDocument,
      {
        ariaLabel: "Terminal actions",
        menuOptions: { shouldForwardArgs: true },
        actionViewItemProvider: (action) => action.id === SELECT_PROFILE_COMMAND_ID
          ? new TerminalProfileActionViewItem(
            action,
            () => this.#profiles,
            () => this.#selectedProfileId,
          )
          : undefined,
      },
    ));
    this.element = this.#toolbar.element;
    this.element.classList.add("zeta-terminal-title-toolbar");
  }

  get selectedProfileId(): string | undefined {
    return this.#selectedProfileId;
  }

  setProfiles(profiles: readonly ITerminalProfile[]): void {
    this.#profiles = profiles;
    if (!profiles.some((profile) => profile.profileId === this.#selectedProfileId)) {
      this.#selectedProfileId = profiles.find((profile) => profile.isDefault)?.profileId ?? profiles[0]?.profileId;
    }
    this.#profilesAvailableContext.set(profiles.length > 0);
    this.#toolbar.refresh();
  }

  setCreating(creating: boolean): void {
    this.#creatingContext.set(creating);
  }

  setActiveInstance(instance: ITerminalInstance | undefined): void {
    this.#hasActiveInstanceContext.set(instance !== undefined);
    this.#activeInstanceStateContext.set(instance?.state ?? "none");
  }

  #registerCommandsAndMenu(options: TerminalTitleActionsOptions): void {
    this.own(CommandsRegistry.register(SELECT_PROFILE_COMMAND_ID, (_accessor, profileId) => {
      if (typeof profileId !== "string" || !this.#profiles.some((profile) => profile.profileId === profileId)) {
        throw new TypeError(`Unknown terminal profile: ${String(profileId)}`);
      }
      this.#selectedProfileId = profileId;
      this.#toolbar.refresh();
    }));
    this.own(CommandsRegistry.register(NEW_TERMINAL_COMMAND_ID, () => options.createTerminal()));
    this.own(CommandsRegistry.register(RELAUNCH_TERMINAL_COMMAND_ID, () => options.relaunchActive()));
    this.own(CommandsRegistry.register(KILL_TERMINAL_COMMAND_ID, () => options.killActive()));
    this.own(MenusRegistry.appendMenuItem(MenuId.TerminalTitle, {
      command: {
        id: SELECT_PROFILE_COMMAND_ID,
        title: "Terminal Profile",
        tooltip: "Select Terminal Profile",
      },
      when: TerminalProfilesAvailableContext.isEqualTo(true),
      group: "navigation",
      order: 0,
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
        icon: lxiconsLibrary.close,
      },
      when: TerminalHasActiveInstanceContext.isEqualTo(true),
      group: "navigation",
      order: 30,
    }));
  }
}

class TerminalProfileActionViewItem extends ActionViewItem {
  #select: HTMLSelectElement | undefined;

  constructor(action: IAction, readonly profiles: () => readonly ITerminalProfile[], readonly selectedProfileId: () => string | undefined) { super(action); }

  override render(container: HTMLElement): void {
    container.classList.add("zeta-terminal-profile-action");
    const select = container.ownerDocument.createElement("select");
    this.#select = select;
    select.className = "zeta-terminal-profile";
    select.setAttribute("aria-label", "Terminal profile");
    select.disabled = !this.action.enabled;
    const selectedProfileId = this.selectedProfileId();
    const options = this.profiles().map((profile) => {
      const option = container.ownerDocument.createElement("option");
      option.value = profile.profileId;
      option.textContent = profile.isDefault ? `${profile.title} (Default)` : profile.title;
      option.selected = profile.profileId === selectedProfileId;
      return option;
    });
    select.append(...options);
    this.own(addDisposableListener(select, "change", () => {
      void this.action.run(select.value);
    }));
    container.append(select);
  }

  override focus(): void {
    this.#requireSelect().focus();
  }

  override setTabbable(tabbable: boolean): void {
    this.#requireSelect().tabIndex = tabbable ? 0 : -1;
  }

  #requireSelect(): HTMLSelectElement {
    if (!this.#select) throw new Error("Terminal profile action is not rendered");
    return this.#select;
  }
}
