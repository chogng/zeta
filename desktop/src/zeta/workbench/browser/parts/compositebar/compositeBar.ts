import "./compositebar.css";
import type { IContextMenuProvider } from "../../../../base/browser/contextmenu.js";
import { Button } from "../../../../base/browser/ui/button/button.js";
import { TabList } from "../../../../base/browser/ui/tablist/tabList.js";
import type { IAction } from "../../../../base/common/actions.js";
import { Emitter, type Event } from "../../../../base/common/event.js";
import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import { lxiconsLibrary } from "../../../../base/common/lxiconsLibrary.js";
import { ViewContainerLocation, type IViewContainerDescriptor } from "../../../common/views.js";
import type { IViewDescriptorService } from "../../../services/views/common/viewDescriptorService.js";

/** Selection of an inactive Composite requested from a CompositeBar. */
export interface CompositeBarSelectionEvent {
  readonly compositeId: string;
}

/** Construction inputs for a location-specific Composite selector. */
export interface CompositeBarOptions {
  readonly ownerDocument: Document;
  readonly viewDescriptorService: IViewDescriptorService;
  readonly location: ViewContainerLocation;
  readonly ariaLabel: string;
  readonly presentation?: CompositeBarPresentation;
  /** Host-owned menu surface used to reveal label tabs that do not fit. */
  readonly contextMenuProvider?: IContextMenuProvider;
}

/** Visual density selected by the Part hosting a CompositeBar. */
export type CompositeBarPresentation = "icon" | "label";

const OVERFLOW_BUTTON_WIDTH = 24;
const LABEL_TAB_LIST_INSET_WIDTH = 16;

/**
 * Maps registered workbench Composites onto the shared TabList.
 *
 * Its containing Part owns construction, activation, visibility, and
 * persisted state for the selected Composite.
 */
export class CompositeBar extends DisposableOwner {
  readonly element: HTMLElement;
  private readonly viewDescriptorService: IViewDescriptorService;
  private readonly location: ViewContainerLocation;
  private readonly tabList: TabList<string>;
  private readonly contextMenuProvider: IContextMenuProvider | undefined;
  private readonly overflowButton: Button | undefined;
  private readonly _onDidSelectComposite =
    this.own(new Emitter<CompositeBarSelectionEvent>());
  private containers: readonly IViewContainerDescriptor[] = [];
  private readonly tabWidths = new Map<string, number>();
  private readonly tabListInsetWidth = LABEL_TAB_LIST_INSET_WIDTH;
  private renderedContainerIds: readonly string[] = [];
  private overflowingContainerIds = new Set<string>();
  private _activeCompositeId: string | undefined;

  readonly onDidSelectComposite: Event<CompositeBarSelectionEvent> =
    this._onDidSelectComposite.event;

  constructor(options: CompositeBarOptions) {
    super();
    this.viewDescriptorService = options.viewDescriptorService;
    this.location = options.location;
    this.contextMenuProvider = options.contextMenuProvider;
    this.element = options.ownerDocument.createElement("section");
    this.element.className = `zeta-composite-bar zeta-composite-bar-${options.presentation ?? "icon"}`;
    this.element.setAttribute("aria-label", options.ariaLabel);
    this.element.dataset.viewContainerLocation = options.location;
    this.defer(() => this.element.remove());
    this.tabList = this.own(new TabList({
      ownerDocument: options.ownerDocument,
      ariaLabel: options.ariaLabel,
      onActivate: (compositeId) => {
        if (this._activeCompositeId === compositeId) return;
        this._onDidSelectComposite.fire({ compositeId });
      },
    }));
    if (options.presentation === "label" && this.contextMenuProvider) {
      const overflowButton = this.own(new Button({
        ownerDocument: options.ownerDocument,
        label: "Additional views",
        title: "Additional views",
        icon: lxiconsLibrary.ellipsis,
        onClick: () => this.showOverflowMenu(),
      }));
      overflowButton.element.classList.add("zeta-composite-bar-overflow");
      overflowButton.element.setAttribute("aria-haspopup", "menu");
      overflowButton.element.setAttribute("aria-expanded", "false");
      overflowButton.element.hidden = true;
      this.overflowButton = overflowButton;
      this.element.append(this.tabList.element, overflowButton.element);
    } else {
      this.overflowButton = undefined;
      this.element.append(this.tabList.element);
    }
    this.own(this.viewDescriptorService.onDidChangeViewContainers(() => {
      this.render();
    }));
    const ResizeObserverConstructor = options.ownerDocument.defaultView?.ResizeObserver;
    if (this.overflowButton && ResizeObserverConstructor) {
      const observer = new ResizeObserverConstructor(() => this.layout());
      observer.observe(this.element);
      this.defer(() => observer.disconnect());
    }
    this.render();
  }

  get activeCompositeId(): string | undefined {
    return this._activeCompositeId;
  }

  setActiveComposite(compositeId: string): void {
    const available = this.viewDescriptorService
      .getViewContainers(this.location)
      .some((container) => container.id === compositeId);
    if (!available) {
      throw new Error(`Composite Bar item is not available: ${compositeId}`);
    }
    if (this._activeCompositeId === compositeId) return;
    this._activeCompositeId = compositeId;
    this.render();
  }

  /** Reconciles visible label tabs with the width assigned by the hosting Part. */
  layout(): void {
    const overflowButton = this.overflowButton;
    if (!overflowButton || !this.measureTabWidths()) return;
    const availableWidth = this.element.clientWidth;
    if (availableWidth <= 0) return;

    const visibleContainers = this.visibleContainersForWidth(availableWidth);
    const visibleContainerIds = visibleContainers.map((container) => container.id);
    if (!sameIds(this.renderedContainerIds, visibleContainerIds)) {
      this.renderTabs(visibleContainers);
    }

    this.overflowingContainerIds = new Set(this.containers
      .filter((container) => !visibleContainerIds.includes(container.id))
      .map((container) => container.id));
    overflowButton.element.hidden = this.overflowingContainerIds.size === 0;
  }

  private render(): void {
    this.containers = this.viewDescriptorService.getViewContainers(
      this.location,
    );
    if (
      this._activeCompositeId !== undefined &&
      !this.containers.some((container) => container.id === this._activeCompositeId)
    ) {
      this._activeCompositeId = undefined;
    }
    this.tabWidths.clear();
    this.overflowingContainerIds.clear();
    if (this.overflowButton) this.overflowButton.element.hidden = true;
    this.renderTabs(this.containers);
    this.layout();
  }

  private renderTabs(containers: readonly IViewContainerDescriptor[]): void {
    this.renderedContainerIds = containers.map((container) => container.id);
    this.tabList.setTabs(
      containers.map((container) => ({
        id: container.id,
        value: container.id,
        label: container.title,
        tooltip: container.title,
        icon: container.icon,
        tabId: compositeTabId(this.location, container.id),
        panelId: compositePanelId(this.location, container.id),
      })),
      this.renderedContainerIds.includes(this._activeCompositeId ?? "")
        ? this._activeCompositeId
        : undefined,
    );
  }

  private measureTabWidths(): boolean {
    const actionBar = this.tabList.element.querySelector<HTMLElement>(
      ".zeta-tab-list-scroll-content > .zeta-action-bar",
    );
    if (!actionBar) return false;
    for (const tab of actionBar.querySelectorAll<HTMLElement>(":scope > .zeta-tab")) {
      const id = tab.dataset.actionId;
      const width = tab.getBoundingClientRect().width;
      if (!id || width <= 0) return false;
      this.tabWidths.set(id, width);
    }
    if (!this.containers.every((container) => this.tabWidths.has(container.id))) {
      return false;
    }
    return true;
  }

  private visibleContainersForWidth(availableWidth: number): readonly IViewContainerDescriptor[] {
    const totalWidth = this.tabListInsetWidth + this.containers.reduce(
      (total, container) => total + this.tabWidths.get(container.id)!,
      0,
    );
    if (totalWidth <= availableWidth) return this.containers;

    const widthLimit = Math.max(0, availableWidth - OVERFLOW_BUTTON_WIDTH);
    const visible: IViewContainerDescriptor[] = [];
    let width = this.tabListInsetWidth;
    for (const container of this.containers) {
      const tabWidth = this.tabWidths.get(container.id)!;
      if (width + tabWidth > widthLimit) break;
      visible.push(container);
      width += tabWidth;
    }

    const activeCompositeId = this._activeCompositeId;
    if (activeCompositeId && !visible.some((container) => container.id === activeCompositeId)) {
      const activeContainer = this.containers.find((container) => container.id === activeCompositeId);
      if (activeContainer) {
        const activeWidth = this.tabWidths.get(activeContainer.id)!;
        while (visible.length > 0 && width + activeWidth > widthLimit) {
          const removed = visible.pop()!;
          width -= this.tabWidths.get(removed.id)!;
        }
        if (width + activeWidth <= widthLimit) visible.push(activeContainer);
      }
    }
    return visible;
  }

  private showOverflowMenu(): void {
    const overflowButton = this.overflowButton;
    const contextMenuProvider = this.contextMenuProvider;
    if (!overflowButton || !contextMenuProvider || this.overflowingContainerIds.size === 0) return;
    overflowButton.element.setAttribute("aria-expanded", "true");
    try {
      contextMenuProvider.showContextMenu({
        anchor: overflowButton.element,
        actions: this.overflowActions(),
        onHide: () => overflowButton.element.setAttribute("aria-expanded", "false"),
      });
    } catch (error) {
      overflowButton.element.setAttribute("aria-expanded", "false");
      throw error;
    }
  }

  private overflowActions(): readonly IAction[] {
    return this.containers
      .filter((container) => this.overflowingContainerIds.has(container.id))
      .map((container) => ({
        id: `zeta.compositeBar.open.${this.location}.${encodeURIComponent(container.id)}`,
        label: container.title,
        tooltip: container.title,
        enabled: true,
        checked: container.id === this._activeCompositeId,
        run: () => {
          if (container.id === this._activeCompositeId) return;
          this._onDidSelectComposite.fire({ compositeId: container.id });
        },
      }));
  }
}

function sameIds(left: readonly string[], right: readonly string[]): boolean {
  return left.length === right.length && left.every((id, index) => id === right[index]);
}

export function compositeTabId(
  location: ViewContainerLocation,
  compositeId: string,
): string {
  return `zeta-${location}-composite-tab-${encodeURIComponent(compositeId)}`;
}

export function compositePanelId(location: ViewContainerLocation, compositeId: string): string {
  return `zeta-${location}-composite-panel-${encodeURIComponent(compositeId)}`;
}
