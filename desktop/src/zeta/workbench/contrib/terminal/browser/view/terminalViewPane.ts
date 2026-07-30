import { addDisposableListener } from "../../../../../base/browser/dom.js";
import { DisposableOwner, ResettableDisposableGroup } from "../../../../../base/common/lifecycle.js";
import type { IMenuService } from "../../../../../platform/actions/common/menuService.js";
import type { IContextKeyService } from "../../../../../platform/contextkey/common/contextkey.js";
import type { IContextMenuService } from "../../../../../platform/contextview/browser/contextMenu.js";
import type { IThemeService } from "../../../../../platform/theme/common/themeService.js";
import { ViewPane, type IViewPaneOptions } from "../../../../browser/parts/views/viewPane.js";
import type { ITerminalDimensions, ITerminalInstance, ITerminalProfileSelection, ITerminalService } from "../../../../services/terminal/common/terminal.js";
import { TerminalInstanceWidget } from "../instance/terminalInstanceWidget.js";
import { TerminalTabsLayout } from "./terminalTabsLayout.js";
import { TerminalTitleActions } from "./terminalTitleActions.js";
import "./media/terminal.css";

const DEFAULT_DIMENSIONS: ITerminalDimensions = { rows: 24, cols: 80 };

/** Tabbed xterm panel whose persistent widgets preserve per-instance terminal state. */
export class TerminalViewPane extends ViewPane {
  private readonly terminalService: ITerminalService;
  private readonly themeService: IThemeService;
  private readonly titleActions: TerminalTitleActions;
  private readonly statusElement: HTMLDivElement;
  private readonly tabsElement: HTMLDivElement;
  private readonly tabsLayout: TerminalTabsLayout;
  private readonly widgetsElement: HTMLDivElement;
  private readonly items = new Map<ITerminalInstance, TerminalViewItem>();
  private readonly tabBindings = this.own(new ResettableDisposableGroup());
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
      createTerminal: () => this.createTerminal(),
      relaunchActive: () => this.relaunchActive(),
      killActive: () => {
        const active = this.terminalService.activeInstance;
        return active ? this.terminalService.closeTerminal(active) : undefined;
      },
    }));

    this.contentElement.classList.add("zeta-terminal-content");
    this.statusElement = options.ownerDocument.createElement("div");
    this.statusElement.className = "zeta-terminal-status";
    this.statusElement.setAttribute("role", "status");
    this.statusElement.hidden = true;
    this.tabsElement = options.ownerDocument.createElement("div");
    this.tabsElement.className = "zeta-terminal-tabs";
    this.tabsElement.setAttribute("role", "tablist");
    this.tabsElement.setAttribute("aria-label", "Terminal instances");
    this.tabsElement.setAttribute("aria-orientation", "vertical");
    this.widgetsElement = options.ownerDocument.createElement("div");
    this.widgetsElement.className = "zeta-terminal-widgets";
    this.tabsLayout = this.own(new TerminalTabsLayout(this.widgetsElement, this.tabsElement));
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

  private async createTerminal(): Promise<void> {
    if (this.creating || this.disposed) return;
    this.creating = true;
    this.titleActions.setCreating(true);
    this.setStatus(undefined);
    try {
      await this.terminalService.createTerminal({
        dimensions: this.activeItem()?.widget.dimensions() ?? DEFAULT_DIMENSIONS,
        profile: this.selectedProfile(),
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
    this.titleActions.setActiveInstance(active);
    for (const [instance, item] of this.items) {
      item.widget.setVisible(instance === active);
    }
    this.renderTabs();
  }

  private renderTabs(): void {
    this.tabBindings.clear();
    const document = this.tabsElement.ownerDocument;
    const active = this.terminalService.activeInstance;
    const tabs = this.terminalService.instances.map((instance) => {
      const tab = document.createElement("span");
      tab.className = "zeta-terminal-tab";
      tab.dataset.state = instance.state;
      if (instance === active) tab.classList.add("active");
      const select = document.createElement("button");
      select.type = "button";
      select.className = "zeta-terminal-tab-select";
      select.setAttribute("role", "tab");
      select.setAttribute("aria-selected", String(instance === active));
      select.setAttribute("aria-label", instance.title);
      const icon = document.createElement("span");
      icon.className = "zeta-terminal-tab-icon";
      icon.setAttribute("aria-hidden", "true");
      icon.textContent = ">_";
      const label = document.createElement("span");
      label.className = "zeta-terminal-tab-label";
      label.textContent = instance.title;
      select.append(icon, label);
      const close = actionButton(document, `Close ${instance.title}`, "×");
      close.classList.add("zeta-terminal-tab-close");
      this.tabBindings.add(addDisposableListener(select, "click", () => {
        this.terminalService.setActiveInstance(instance);
        this.focus();
      }));
      this.tabBindings.add(addDisposableListener(close, "click", () => {
        void this.terminalService.closeTerminal(instance).catch(() => {});
      }));
      tab.append(select, close);
      return tab;
    });
    this.tabsElement.replaceChildren(...tabs);
  }

  private selectedProfile(): ITerminalProfileSelection {
    const profileId = this.titleActions.selectedProfileId;
    return profileId ? { type: "profile", profileId } : { type: "default" };
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

function actionButton(document: Document, label: string, text: string): HTMLButtonElement {
  const button = document.createElement("button");
  button.type = "button";
  button.className = "zeta-terminal-action";
  button.setAttribute("aria-label", label);
  button.title = label;
  button.textContent = text;
  return button;
}
