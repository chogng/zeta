import { TabList } from "../../../../../base/browser/ui/tablist/tabList.js";
import { DisposableOwner } from "../../../../../base/common/lifecycle.js";
import { lxiconsLibrary } from "../../../../../base/common/lxiconsLibrary.js";
import type { IAction } from "../../../../../base/common/actions.js";
import type { IMenuService } from "../../../../../platform/actions/common/menuService.js";
import type { IContextKeyService } from "../../../../../platform/contextkey/common/contextkey.js";
import type { IContextMenuService } from "../../../../../platform/contextview/browser/contextMenu.js";
import type { IThemeService } from "../../../../../platform/theme/common/themeService.js";
import { ViewPane, type IViewPaneOptions } from "../../../../browser/parts/views/viewPane.js";
import type { ITerminalDimensions, ITerminalInstance, ITerminalService } from "../../../../services/terminal/common/terminal.js";
import { TerminalInstanceWidget } from "../instance/terminalInstanceWidget.js";
import { TerminalTabsLayout } from "./terminalTabsLayout.js";
import { terminalProfileIcon } from "./terminalProfileIcon.js";
import { TerminalTitleActions } from "./terminalTitleActions.js";
import "./media/terminal.css";

const DEFAULT_DIMENSIONS: ITerminalDimensions = { rows: 24, cols: 80 };

/** Tabbed xterm panel whose persistent widgets preserve per-instance terminal state. */
export class TerminalViewPane extends ViewPane {
  private readonly terminalService: ITerminalService;
  private readonly themeService: IThemeService;
  private readonly titleActions: TerminalTitleActions;
  private readonly statusElement: HTMLDivElement;
  private readonly tabList: TabList<ITerminalInstance>;
  private readonly tabsLayout: TerminalTabsLayout;
  private readonly widgetsElement: HTMLDivElement;
  private readonly items = new Map<ITerminalInstance, TerminalViewItem>();
  private creating = false;
  private disposed = false;

  constructor(options: IViewPaneOptions, terminalService: ITerminalService, themeService: IThemeService, menuService: IMenuService, contextMenuService: IContextMenuService, contextKeyService: IContextKeyService) {
    super(options);
    this.defer(() => {
      this.disposed = true;
    });
    this.terminalService = terminalService;
    this.themeService = themeService;
    this.element.classList.add("zeta-terminal-view");
    this.titleElement.remove();
    this.titleActions = this.own(new TerminalTitleActions({
      ownerDocument: options.ownerDocument,
      menuService,
      contextMenuService,
      contextKeyService,
      createTerminal: (profileId) => this.createTerminal(profileId),
      focusActive: () => this.focus(),
      relaunchActive: () => this.relaunchActive(),
      killActive: () => {
        const active = this.terminalService.activeInstance;
        return active ? this.terminalService.closeTerminal(active) : undefined;
      },
      clearActive: () => this.clearActive(),
    }));

    this.contentElement.classList.add("zeta-terminal-content");
    this.statusElement = options.ownerDocument.createElement("div");
    this.statusElement.className = "zeta-terminal-status";
    this.statusElement.setAttribute("role", "status");
    this.statusElement.hidden = true;
    this.tabList = this.own(new TabList({
      ownerDocument: options.ownerDocument,
      ariaLabel: "Terminal instances",
      orientation: "vertical",
      closeActionIcon: lxiconsLibrary.trash,
      onActivate: (instance) => {
        this.terminalService.setActiveInstance(instance);
        this.focus();
      },
      onClose: (instance) => {
        void this.terminalService.closeTerminal(instance).catch(() => {});
      },
    }));
    this.tabList.element.classList.add("zeta-terminal-tabs");
    this.widgetsElement = options.ownerDocument.createElement("div");
    this.widgetsElement.className = "zeta-terminal-widgets";
    this.tabsLayout = this.own(new TerminalTabsLayout(this.widgetsElement, this.tabList.element));
    this.contentElement.append(this.statusElement, this.tabsLayout.element);

    for (const instance of terminalService.instances) this.addInstance(instance);
    this.own(terminalService.onDidCreateInstance((instance) => {
      this.addInstance(instance);
      this.render();
    }));
    this.own(terminalService.onDidDisposeInstance((instance) => {
      this.removeInstance(instance);
      this.render();
    }));
    this.own(terminalService.onDidChangeActiveInstance(() => this.render()));

    const ResizeObserverConstructor = options.ownerDocument.defaultView?.ResizeObserver;
    if (ResizeObserverConstructor) {
      const observer = new ResizeObserverConstructor(() => {
        const bounds = this.tabsLayout.element.getBoundingClientRect();
        this.tabsLayout.layout(bounds.width, bounds.height);
        this.activeItem()?.widget.fit();
      });
      observer.observe(this.tabsLayout.element);
      observer.observe(this.widgetsElement);
      this.defer(() => observer.disconnect());
    }
    this.render();
    queueMicrotask(() => {
      if (!this.disposed) void this.initialize();
    });
  }

  override focus(): void {
    this.activeItem()?.widget.focus();
  }

  override get titleActionsElement(): HTMLElement {
    return this.titleActions.element;
  }

  override setTitleSecondaryActions(actions: readonly IAction[]): boolean {
    this.titleActions.setSupplementalSecondaryActions(actions);
    return true;
  }

  private async initialize(): Promise<void> {
    try {
      const profiles = await this.terminalService.getProfiles();
      if (this.disposed) return;
      this.titleActions.setProfiles(profiles);
    } catch {
      if (this.disposed) return;
      this.titleActions.setProfiles([]);
    }
    if (!this.terminalService.activeInstance) await this.createTerminal();
  }

  private async createTerminal(profileId?: string): Promise<void> {
    if (this.creating || this.disposed) return;
    this.creating = true;
    this.titleActions.setCreating(true);
    this.setStatus(undefined);
    try {
      await this.terminalService.createTerminal({
        dimensions: this.activeItem()?.widget.dimensions() ?? DEFAULT_DIMENSIONS,
        profile: profileId ? { type: "profile", profileId } : { type: "default" },
      });
      if (!this.disposed) this.focus();
    } catch (error) {
      if (!this.disposed) {
        this.setStatus(error instanceof Error ? error.message : "Terminal is unavailable");
      }
    } finally {
      this.creating = false;
      if (!this.disposed) this.titleActions.setCreating(false);
    }
  }

  private async relaunchActive(): Promise<void> {
    const instance = this.terminalService.activeInstance;
    const item = instance ? this.items.get(instance) : undefined;
    if (!instance || !item || instance.state === "running") return;
    this.setStatus(undefined);
    try {
      await this.terminalService.relaunchTerminal(instance, item.widget.dimensions());
      if (!this.disposed) item.widget.focus();
    } catch (error) {
      if (!this.disposed) {
        this.setStatus(error instanceof Error ? error.message : "Terminal relaunch failed");
      }
    }
  }

  private clearActive(): void {
    this.activeItem()?.widget.clear();
    this.focus();
  }

  private addInstance(instance: ITerminalInstance): void {
    if (this.items.has(instance)) return;
    const item = this.own(new TerminalViewItem(
      instance,
      new TerminalInstanceWidget(instance, this.element.ownerDocument, this.themeService),
      () => this.render(),
    ));
    this.items.set(instance, item);
    this.widgetsElement.append(item.widget.element);
  }

  private removeInstance(instance: ITerminalInstance): void {
    const item = this.items.get(instance);
    if (!item) return;
    this.items.delete(instance);
    item.dispose();
  }

  private render(): void {
    if (this.disposed) return;
    const active = this.terminalService.activeInstance;
    const instanceSwitcherPlacement = this.terminalService.instances.length > 1 ? "list" : "title";
    this.titleActions.setActiveInstance(active, instanceSwitcherPlacement);
    this.tabsLayout.setInstanceListPresentation(instanceSwitcherPlacement === "list" ? "visible" : "hidden");
    for (const [instance, item] of this.items) {
      item.widget.setVisible(instance === active);
    }
    this.renderTabs();
  }

  private renderTabs(): void {
    const active = this.terminalService.activeInstance;
    this.tabList.setTabs(this.terminalService.instances.map((instance) => ({
      id: instance.id,
      value: instance,
      label: instance.title,
      tooltip: instance.title,
      icon: terminalProfileIcon(instance.profile),
      state: instance.state,
      tabId: `${instance.id}-tab`,
    })), active?.id);
  }

  private activeItem(): TerminalViewItem | undefined {
    const active = this.terminalService.activeInstance;
    return active ? this.items.get(active) : undefined;
  }

  private setStatus(message: string | undefined): void {
    this.statusElement.textContent = message ?? "";
    this.statusElement.hidden = message === undefined;
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
