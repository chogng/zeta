import { addDisposableListener } from "../../../../base/browser/dom.js";
import { ActionViewItem } from "../../../../base/browser/ui/actionbar/actionViewItems.js";
import { ToolBar } from "../../../../base/browser/ui/toolbar/toolbar.js";
import type { IAction } from "../../../../base/common/actions.js";
import type { Icon } from "../../../../base/common/icon.js";
import { LxIcon } from "../../../../base/common/lxicons.js";
import { DisposableOwner, ResettableDisposableGroup } from "../../../../base/common/lifecycle.js";
import type { IThemeService } from "../../../../platform/theme/common/themeService.js";
import { ViewPane, type IViewPaneOptions } from "../../../browser/parts/views/viewPane.js";
import type { ITerminalDimensions, ITerminalInstance, ITerminalProfile, ITerminalProfileSelection, ITerminalService } from "../common/terminal.js";
import { TerminalInstanceWidget } from "./terminalInstanceWidget.js";
import "./media/terminal.css";

const DEFAULT_DIMENSIONS: ITerminalDimensions = { rows: 24, cols: 80 };

/** Tabbed xterm panel whose persistent widgets preserve per-instance terminal state. */
export class TerminalViewPane extends ViewPane {
  readonly #terminalService: ITerminalService;
  readonly #themeService: IThemeService;
  readonly #titleToolbar: ToolBar;
  readonly #statusElement: HTMLDivElement;
  readonly #tabsElement: HTMLDivElement;
  readonly #widgetsElement: HTMLDivElement;
  readonly #items = new Map<ITerminalInstance, TerminalViewItem>();
  readonly #tabBindings = this.own(new ResettableDisposableGroup());
  #profiles: readonly ITerminalProfile[] = [];
  #selectedProfileId: string | undefined;
  #creating = false;
  #disposed = false;

  constructor(options: IViewPaneOptions, terminalService: ITerminalService, themeService: IThemeService) {
    super(options);
    this.defer(() => {
      this.#disposed = true;
    });
    this.#terminalService = terminalService;
    this.#themeService = themeService;
    this.element.classList.add("zeta-terminal-view");
    this.titleElement.remove();
    this.#titleToolbar = this.own(new ToolBar({
      contextMenuProvider: noTerminalContextMenu,
      ownerDocument: options.ownerDocument,
      ariaLabel: "Terminal actions",
      actionViewItemProvider: (action) => action instanceof TerminalProfileAction
        ? new TerminalProfileActionViewItem(action)
        : undefined,
    }));
    this.#titleToolbar.element.classList.add("zeta-terminal-title-toolbar");

    this.contentElement.classList.add("zeta-terminal-content");
    this.#statusElement = options.ownerDocument.createElement("div");
    this.#statusElement.className = "zeta-terminal-status";
    this.#statusElement.setAttribute("role", "status");
    this.#statusElement.hidden = true;
    this.#tabsElement = options.ownerDocument.createElement("div");
    this.#tabsElement.className = "zeta-terminal-tabs";
    this.#tabsElement.setAttribute("role", "tablist");
    this.#widgetsElement = options.ownerDocument.createElement("div");
    this.#widgetsElement.className = "zeta-terminal-widgets";
    this.contentElement.append(this.#statusElement, this.#tabsElement, this.#widgetsElement);

    for (const instance of terminalService.instances) this.#addInstance(instance);
    this.own(terminalService.onDidCreateInstance((instance) => {
      this.#addInstance(instance);
      this.#render();
    }));
    this.own(terminalService.onDidDisposeInstance((instance) => {
      this.#removeInstance(instance);
      this.#render();
    }));
    this.own(terminalService.onDidChangeActiveInstance(() => this.#render()));

    const ResizeObserverConstructor = options.ownerDocument.defaultView?.ResizeObserver;
    if (ResizeObserverConstructor) {
      const observer = new ResizeObserverConstructor(() => this.#activeItem()?.widget.fit());
      observer.observe(this.#widgetsElement);
      this.defer(() => observer.disconnect());
    }
    this.#render();
    queueMicrotask(() => {
      if (!this.#disposed) void this.#initialize();
    });
  }

  override focus(): void {
    this.#activeItem()?.widget.focus();
  }

  override get titleActionsElement(): HTMLElement {
    return this.#titleToolbar.element;
  }

  async #initialize(): Promise<void> {
    try {
      this.#profiles = await this.#terminalService.getProfiles();
      if (this.#disposed) return;
      this.#selectedProfileId = this.#profiles.find((profile) => profile.isDefault)?.profileId ?? this.#profiles[0]?.profileId;
      this.#renderTitleToolbar();
    } catch {
      this.#profiles = [];
    }
    if (!this.#terminalService.activeInstance) await this.#createTerminal();
  }

  async #createTerminal(): Promise<void> {
    if (this.#creating || this.#disposed) return;
    this.#creating = true;
    this.#renderTitleToolbar();
    this.#setStatus(undefined);
    try {
      await this.#terminalService.createTerminal({
        dimensions: this.#activeItem()?.widget.dimensions() ?? DEFAULT_DIMENSIONS,
        profile: this.#selectedProfile(),
      });
      if (!this.#disposed) this.focus();
    } catch (error) {
      if (!this.#disposed) {
        this.#setStatus(error instanceof Error ? error.message : "Terminal is unavailable");
      }
    } finally {
      this.#creating = false;
      if (!this.#disposed) this.#renderTitleToolbar();
    }
  }

  async #relaunchActive(): Promise<void> {
    const instance = this.#terminalService.activeInstance;
    const item = instance ? this.#items.get(instance) : undefined;
    if (!instance || !item || instance.state === "running") return;
    this.#setStatus(undefined);
    try {
      await this.#terminalService.relaunchTerminal(instance, item.widget.dimensions());
      if (!this.#disposed) item.widget.focus();
    } catch (error) {
      if (!this.#disposed) {
        this.#setStatus(error instanceof Error ? error.message : "Terminal relaunch failed");
      }
    }
  }

  #addInstance(instance: ITerminalInstance): void {
    if (this.#items.has(instance)) return;
    const item = this.own(new TerminalViewItem(
      instance,
      new TerminalInstanceWidget(instance, this.element.ownerDocument, this.#themeService),
      () => this.#render(),
    ));
    this.#items.set(instance, item);
    this.#widgetsElement.append(item.widget.element);
  }

  #removeInstance(instance: ITerminalInstance): void {
    const item = this.#items.get(instance);
    if (!item) return;
    this.#items.delete(instance);
    item.dispose();
  }

  #render(): void {
    if (this.#disposed) return;
    const active = this.#terminalService.activeInstance;
    for (const [instance, item] of this.#items) {
      item.widget.setVisible(instance === active);
    }
    this.#renderTabs();
    this.#renderTitleToolbar();
  }

  #renderTabs(): void {
    this.#tabBindings.clear();
    const document = this.#tabsElement.ownerDocument;
    const active = this.#terminalService.activeInstance;
    const tabs = this.#terminalService.instances.map((instance) => {
      const tab = document.createElement("span");
      tab.className = "zeta-terminal-tab";
      tab.dataset.state = instance.state;
      if (instance === active) tab.classList.add("active");
      const select = document.createElement("button");
      select.type = "button";
      select.className = "zeta-terminal-tab-select";
      select.setAttribute("role", "tab");
      select.setAttribute("aria-selected", String(instance === active));
      select.textContent = instance.title;
      const close = actionButton(document, `Close ${instance.title}`, "×");
      close.classList.add("zeta-terminal-tab-close");
      this.#tabBindings.add(addDisposableListener(select, "click", () => {
        this.#terminalService.setActiveInstance(instance);
        this.focus();
      }));
      this.#tabBindings.add(addDisposableListener(close, "click", () => {
        void this.#terminalService.closeTerminal(instance).catch(() => {});
      }));
      tab.append(select, close);
      return tab;
    });
    this.#tabsElement.replaceChildren(...tabs);
  }

  #renderTitleToolbar(): void {
    const active = this.#terminalService.activeInstance;
    const actions: IAction[] = [];
    if (this.#profiles.length > 0) {
      actions.push(new TerminalProfileAction(
        this.#profiles,
        this.#selectedProfileId,
        (profileId) => {
          this.#selectedProfileId = profileId;
        },
      ));
    }
    actions.push(terminalAction(
      "zeta.terminal.new",
      "New Terminal",
      LxIcon.add,
      !this.#creating,
      () => this.#createTerminal(),
    ));
    if (active && active.state !== "running") {
      actions.push(terminalAction(
        "zeta.terminal.relaunch",
        "Relaunch Terminal",
        LxIcon.history,
        true,
        () => this.#relaunchActive(),
      ));
    }
    if (active) {
      actions.push(terminalAction(
        "zeta.terminal.kill",
        "Kill Terminal",
        LxIcon.close,
        true,
        () => this.#terminalService.closeTerminal(active),
      ));
    }
    this.#titleToolbar.setActions(actions);
  }

  #selectedProfile(): ITerminalProfileSelection {
    const profileId = this.#selectedProfileId;
    return profileId ? { type: "profile", profileId } : { type: "default" };
  }

  #activeItem(): TerminalViewItem | undefined {
    const active = this.#terminalService.activeInstance;
    return active ? this.#items.get(active) : undefined;
  }

  #setStatus(message: string | undefined): void {
    this.#statusElement.textContent = message ?? "";
    this.#statusElement.hidden = message === undefined;
  }
}

class TerminalProfileAction implements IAction {
  readonly id = "zeta.terminal.selectProfile";
  readonly label = "Terminal Profile";
  readonly tooltip = "Select Terminal Profile";
  readonly enabled = true;
  readonly checked = undefined;

  constructor(
    readonly profiles: readonly ITerminalProfile[],
    readonly selectedProfileId: string | undefined,
    readonly selectProfile: (profileId: string) => void,
  ) {}

  run(): void {}
}

class TerminalProfileActionViewItem extends ActionViewItem {
  #select: HTMLSelectElement | undefined;

  constructor(readonly profileAction: TerminalProfileAction) {
    super(profileAction);
  }

  override render(container: HTMLElement): void {
    container.classList.add("zeta-terminal-profile-action");
    const select = container.ownerDocument.createElement("select");
    this.#select = select;
    select.className = "zeta-terminal-profile";
    select.setAttribute("aria-label", "Terminal profile");
    const options = this.profileAction.profiles.map((profile) => {
      const option = container.ownerDocument.createElement("option");
      option.value = profile.profileId;
      option.textContent = profile.isDefault ? `${profile.title} (Default)` : profile.title;
      option.selected = profile.profileId === this.profileAction.selectedProfileId;
      return option;
    });
    select.append(...options);
    this.own(addDisposableListener(select, "change", () => {
      this.profileAction.selectProfile(select.value);
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

class TerminalViewItem extends DisposableOwner {
  constructor(
    readonly instance: ITerminalInstance,
    readonly widget: TerminalInstanceWidget,
    onDidChangeState: () => void,
  ) {
    super();
    this.own(widget);
    this.own(instance.onDidChangeState(onDidChangeState));
  }
}

function actionButton(document: Document, label: string, text: string): HTMLButtonElement {
  const button = document.createElement("button");
  button.type = "button";
  button.className = "zeta-terminal-action";
  button.setAttribute("aria-label", label);
  button.title = label;
  button.textContent = text;
  return button;
}

function terminalAction(id: string, label: string, icon: Icon, enabled: boolean, run: () => unknown): IAction {
  return { id, label, tooltip: label, icon, enabled, checked: undefined, run };
}

const noTerminalContextMenu = {
  showContextMenu(): never {
    throw new Error("Terminal title toolbar has no secondary actions");
  },
};
