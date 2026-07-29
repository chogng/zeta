import "./compositebar.css";
import { TabList } from "../../../../base/browser/ui/tablist/tabList.js";
import { Emitter, type Event } from "../../../../base/common/event.js";
import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import { ViewContainerLocation } from "../../../common/views.js";
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
}

/**
 * Maps registered workbench Composites onto the shared TabList.
 *
 * Its containing Part owns construction, activation, visibility, and
 * persisted state for the selected Composite.
 */
export class CompositeBar extends DisposableOwner {
  readonly element: HTMLElement;
  readonly #viewDescriptorService: IViewDescriptorService;
  readonly #location: ViewContainerLocation;
  readonly #tabList: TabList<string>;
  readonly #onDidSelectComposite =
    this.own(new Emitter<CompositeBarSelectionEvent>());
  #activeCompositeId: string | undefined;

  readonly onDidSelectComposite: Event<CompositeBarSelectionEvent> =
    this.#onDidSelectComposite.event;

  constructor(options: CompositeBarOptions) {
    super();
    this.#viewDescriptorService = options.viewDescriptorService;
    this.#location = options.location;
    this.element = options.ownerDocument.createElement("section");
    this.element.className = "zeta-composite-bar";
    this.element.setAttribute("aria-label", options.ariaLabel);
    this.element.dataset.viewContainerLocation = options.location;
    this.defer(() => this.element.remove());
    this.#tabList = this.own(new TabList({
      ownerDocument: options.ownerDocument,
      ariaLabel: options.ariaLabel,
      onActivate: (compositeId) => {
        if (this.#activeCompositeId === compositeId) return;
        this.#onDidSelectComposite.fire({ compositeId });
      },
    }));
    this.element.append(this.#tabList.element);
    this.own(this.#viewDescriptorService.onDidChangeViewContainers(() => {
      this.#render();
    }));
    this.#render();
  }

  get activeCompositeId(): string | undefined {
    return this.#activeCompositeId;
  }

  setActiveComposite(compositeId: string): void {
    const available = this.#viewDescriptorService
      .getViewContainers(this.#location)
      .some((container) => container.id === compositeId);
    if (!available) {
      throw new Error(`Composite Bar item is not available: ${compositeId}`);
    }
    if (this.#activeCompositeId === compositeId) return;
    this.#activeCompositeId = compositeId;
    this.#render();
  }

  #render(): void {
    const containers = this.#viewDescriptorService.getViewContainers(
      this.#location,
    );
    if (
      this.#activeCompositeId !== undefined &&
      !containers.some((container) => container.id === this.#activeCompositeId)
    ) {
      this.#activeCompositeId = undefined;
    }
    this.#tabList.setTabs(
      containers.map((container) => ({
        id: container.id,
        value: container.id,
        label: container.title,
        tooltip: container.title,
        icon: container.icon,
        tabId: compositeTabId(this.#location, container.id),
        panelId: compositePanelId(this.#location, container.id),
      })),
      this.#activeCompositeId,
    );
  }
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
