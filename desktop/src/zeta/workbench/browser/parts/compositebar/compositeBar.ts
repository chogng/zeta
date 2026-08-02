import "./compositebar.css";
import type { IContextMenuProvider } from "../../../../base/browser/contextmenu.js";
import type { ActionViewItem } from "../../../../base/browser/ui/actionbar/actionViewItems.js";
import { DropdownMenuActionViewItem } from "../../../../base/browser/ui/dropdown/dropdownMenuActionViewItem.js";
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
  /** Selects the View Containers represented as Composite Bar action items. */
  readonly containerFilter?: (container: IViewContainerDescriptor) => boolean;
  /** Host-owned menu surface used to reveal label tabs that do not fit. */
  readonly contextMenuProvider?: IContextMenuProvider;
}

/** Visual density selected by the Part hosting a CompositeBar. */
export type CompositeBarPresentation = "icon" | "label";

const OVERFLOW_BUTTON_WIDTH = 24;
const OVERFLOW_ACTION_ID = "zeta.compositeBar.overflow";

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
  private readonly containerFilter: (container: IViewContainerDescriptor) => boolean;
  private overflowViewItem: CompositeBarOverflowViewItem | undefined;
  private readonly _onDidSelectComposite =
    this.own(new Emitter<CompositeBarSelectionEvent>());
  private containers: readonly IViewContainerDescriptor[] = [];
  private readonly tabWidths = new Map<string, number>();
  private tabListInsetWidth = 0;
  private tabListItemGap = 0;
  private renderedContainerIds: readonly string[] = [];
  private overflowingContainerIds = new Set<string>();
  private _activeCompositeId: string | undefined;

  readonly onDidSelectComposite: Event<CompositeBarSelectionEvent> =
    this._onDidSelectComposite.event;

  constructor(options: CompositeBarOptions) {
    super();
    const presentation = options.presentation ?? "icon";
    this.viewDescriptorService = options.viewDescriptorService;
    this.location = options.location;
    this.contextMenuProvider = options.contextMenuProvider;
    this.containerFilter = options.containerFilter ?? (() => true);
    this.element = options.ownerDocument.createElement("section");
    this.element.className = `zeta-composite-bar zeta-composite-bar-${presentation}`;
    this.element.setAttribute("aria-label", options.ariaLabel);
    this.element.dataset.viewContainerLocation = options.location;
    this.defer(() => this.element.remove());
    const overflowAction = presentation === "label" && this.contextMenuProvider
      ? new CompositeBarOverflowAction()
      : undefined;
    this.tabList = this.own(new TabList({
      ownerDocument: options.ownerDocument,
      ariaLabel: options.ariaLabel,
      presentation: presentation === "icon" ? "inset" : "flush",
      trailingActions: overflowAction ? [overflowAction] : undefined,
      trailingActionViewItemProvider: overflowAction
        ? (action): ActionViewItem => {
          const item = new CompositeBarOverflowViewItem(
            action,
            () => this.createOverflowActions(),
            this.contextMenuProvider!,
          );
          this.overflowViewItem = item;
          return item;
        }
        : undefined,
      onActivate: (compositeId) => {
        if (this._activeCompositeId === compositeId) return;
        this._onDidSelectComposite.fire({ compositeId });
      },
    }));
    this.element.append(this.tabList.element);
    this.own(this.viewDescriptorService.onDidChangeViewContainers(() => {
      this.render();
    }));
    const ResizeObserverConstructor = options.ownerDocument.defaultView?.ResizeObserver;
    if (overflowAction && ResizeObserverConstructor) {
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
    const overflowViewItem = this.overflowViewItem;
    if (!overflowViewItem || !this.measureTabWidths()) return;
    const availableWidth = this.element.clientWidth;
    if (availableWidth <= 0) return;

    const visibleContainers = this.visibleContainersForWidth(availableWidth, OVERFLOW_BUTTON_WIDTH + this.tabListItemGap);
    const visibleContainerIds = visibleContainers.map((container) => container.id);
    if (!sameIds(this.renderedContainerIds, visibleContainerIds)) {
      this.renderTabs(visibleContainers);
    }

    this.setOverflowingContainerIds(new Set(this.containers
      .filter((container) => !visibleContainerIds.includes(container.id))
      .map((container) => container.id)));
    if (this.overflowViewItem) {
      this.overflowViewItem.hidden = this.overflowingContainerIds.size === 0;
    }
  }

  private render(): void {
    const availableContainers = this.viewDescriptorService.getViewContainers(this.location);
    this.containers = availableContainers.filter(this.containerFilter);
    if (
      this._activeCompositeId !== undefined &&
      !availableContainers.some((container) => container.id === this._activeCompositeId)
    ) {
      this._activeCompositeId = undefined;
    }
    this.tabWidths.clear();
    this.setOverflowingContainerIds(new Set());
    if (this.overflowViewItem) this.overflowViewItem.hidden = true;
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
    if (this.overflowViewItem) {
      this.overflowViewItem.hidden = this.overflowingContainerIds.size === 0;
    }
  }

  private measureTabWidths(): boolean {
    const actionBar = this.tabList.element.querySelector<HTMLElement>(
      ".zeta-tab-list-scroll-content > .zeta-action-bar",
    );
    if (!actionBar) return false;
    const tabs = [...actionBar.querySelectorAll<HTMLElement>(":scope > .zeta-tab")];
    const tabBounds: DOMRect[] = [];
    let totalTabWidth = 0;
    for (const tab of tabs) {
      const id = tab.dataset.actionId;
      const bounds = tab.getBoundingClientRect();
      const width = bounds.width;
      if (!id || width <= 0) return false;
      this.tabWidths.set(id, width);
      tabBounds.push(bounds);
      totalTabWidth += width;
    }
    const firstTabBounds = tabBounds[0];
    const lastTabBounds = tabBounds.at(-1);
    if (firstTabBounds && lastTabBounds) {
      const actionBarBounds = actionBar.getBoundingClientRect();
      this.tabListInsetWidth = Math.max(0, firstTabBounds.left - actionBarBounds.left) * 2;
      if (tabBounds.length > 1) {
        const itemSpan = lastTabBounds.right - firstTabBounds.left;
        this.tabListItemGap = Math.max(0, (itemSpan - totalTabWidth) / (tabBounds.length - 1));
      }
    }
    if (!this.containers.every((container) => this.tabWidths.has(container.id))) {
      return false;
    }
    return true;
  }

  private visibleContainersForWidth(availableWidth: number, overflowWidth: number): readonly IViewContainerDescriptor[] {
    const totalWidth = this.containersWidth(this.containers);
    if (totalWidth <= availableWidth) return this.containers;

    const widthLimit = Math.max(0, availableWidth - overflowWidth);
    const visible: IViewContainerDescriptor[] = [];
    for (const container of this.containers) {
      if (this.containersWidth([...visible, container]) > widthLimit) break;
      visible.push(container);
    }

    const activeCompositeId = this._activeCompositeId;
    if (activeCompositeId && !visible.some((container) => container.id === activeCompositeId)) {
      const activeContainer = this.containers.find((container) => container.id === activeCompositeId);
      if (activeContainer) {
        while (visible.length > 0 && this.containersWidth([...visible, activeContainer]) > widthLimit) visible.pop();
        if (this.containersWidth([...visible, activeContainer]) <= widthLimit) visible.push(activeContainer);
      }
    }
    return visible;
  }

  private containersWidth(containers: readonly IViewContainerDescriptor[]): number {
    return this.tabListInsetWidth + containers.reduce(
      (total, container, index) => total + this.tabWidths.get(container.id)! + (index > 0 ? this.tabListItemGap : 0),
      0,
    );
  }

  private createOverflowActions(): readonly IAction[] {
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

  private setOverflowingContainerIds(ids: Set<string>): void {
    if (sameIds([...this.overflowingContainerIds], [...ids])) return;
    this.overflowingContainerIds = ids;
  }
}

class CompositeBarOverflowAction implements IAction {
  readonly id = OVERFLOW_ACTION_ID;
  readonly label = "Additional views";
  readonly tooltip = "Additional views";
  readonly icon = lxiconsLibrary.ellipsis;
  readonly enabled = true;

  run(): void {}
}

class CompositeBarOverflowViewItem extends DropdownMenuActionViewItem {
  private container: HTMLElement | undefined;

  override render(container: HTMLElement): void {
    super.render(container);
    this.container = container;
    container.classList.add("zeta-composite-bar-overflow");
  }

  set hidden(hidden: boolean) {
    if (!this.container) return;
    this.container.hidden = hidden;
    this.container.classList.toggle("hidden", hidden);
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
