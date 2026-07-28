import "./activitybarpart.css";
import "./activityaction.css";
import {
  ActionBar,
} from "../../../../base/browser/ui/actionbar/actionbar.js";
import type { IAction } from "../../../../base/common/actions.js";
import { Emitter, type Event } from "../../../../base/common/event.js";
import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import type {
  IViewContainerDescriptor,
} from "../../../common/views.js";
import { ViewContainerLocation } from "../../../common/views.js";
import type {
  IViewDescriptorService,
} from "../../../services/views/common/viewDescriptorService.js";
import {
  ActivitybarActionViewItem,
} from "./activitybarActions.js";

/**
 * Selection of an inactive View Container requested from the Activity Bar.
 *
 * Clicking the already active item is intentionally a no-op; Sidebar
 * visibility remains owned by the Title Bar command.
 */
export interface ActivitybarSelectionEvent {
  readonly viewContainerId: string;
}

export interface ActivitybarPartOptions {
  readonly ownerDocument: Document;
  readonly viewDescriptorService: IViewDescriptorService;
}

/**
 * Sidebar-owned container for the generic ActionBar that switches primary
 * View Containers.
 */
export class ActivitybarPart extends DisposableOwner {
  readonly element: HTMLElement;
  readonly #viewDescriptorService: IViewDescriptorService;
  readonly #actionbar: ActionBar;
  readonly #items = new Map<string, ActivitybarActionViewItem>();
  readonly #onDidSelectViewContainer =
    this.own(new Emitter<ActivitybarSelectionEvent>());
  #activeViewContainerId: string | undefined;

  readonly onDidSelectViewContainer: Event<ActivitybarSelectionEvent> =
    this.#onDidSelectViewContainer.event;

  constructor(options: ActivitybarPartOptions) {
    super();
    this.#viewDescriptorService = options.viewDescriptorService;
    this.element = options.ownerDocument.createElement("section");
    this.element.className = "zeta-activitybar-container";
    this.element.setAttribute("aria-label", "Activity Bar");
    this.defer(() => this.element.remove());
    this.#actionbar = this.own(new ActionBar({
      ownerDocument: options.ownerDocument,
      ariaRole: "tablist",
      ariaLabel: "Primary side bar views",
      actionViewItemProvider: (action) => {
        const item = new ActivitybarActionViewItem(action);
        this.#items.set(action.id, item);
        return item;
      },
    }));
    this.element.append(this.#actionbar.element);
    this.own(this.#viewDescriptorService.onDidChangeViewContainers(() => {
      this.#render();
    }));
    this.#render();
  }

  get activeViewContainerId(): string | undefined {
    return this.#activeViewContainerId;
  }

  setActiveViewContainer(viewContainerId: string): void {
    if (!this.#items.has(viewContainerId)) {
      throw new Error(
        `Activity Bar View Container is not available: ${viewContainerId}`,
      );
    }
    if (this.#activeViewContainerId === viewContainerId) return;
    this.#activeViewContainerId = viewContainerId;
    this.#updateCheckedState();
  }

  #render(): void {
    this.#items.clear();
    const containers = this.#viewDescriptorService.getViewContainers(
      ViewContainerLocation.Sidebar,
    );
    this.#actionbar.setActions(
      containers.map((container) => this.#createAction(container)),
    );
    for (const container of containers) {
      if (!this.#items.has(container.id)) {
        throw new Error(
          `Activity Bar action did not render: ${container.id}`,
        );
      }
    }
    if (
      this.#activeViewContainerId !== undefined &&
      !this.#items.has(this.#activeViewContainerId)
    ) {
      this.#activeViewContainerId = undefined;
    }
    this.#updateCheckedState();
  }

  #createAction(
    container: IViewContainerDescriptor,
  ): IAction {
    return {
      id: container.id,
      label: container.title,
      tooltip: container.title,
      icon: container.icon,
      enabled: true,
      checked: container.id === this.#activeViewContainerId,
      run: () => {
        if (this.#activeViewContainerId === container.id) return;
        this.#onDidSelectViewContainer.fire({
          viewContainerId: container.id,
        });
      },
    };
  }

  #updateCheckedState(): void {
    for (const [viewContainerId, item] of this.#items) {
      item.setActive(viewContainerId === this.#activeViewContainerId);
    }
  }
}
