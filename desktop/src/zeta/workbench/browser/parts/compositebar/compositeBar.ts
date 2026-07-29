import "./compositebar.css";
import {
  ActionBar,
} from "../../../../base/browser/ui/actionbar/actionbar.js";
import type { IAction } from "../../../../base/common/actions.js";
import { Emitter, type Event } from "../../../../base/common/event.js";
import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import type {
  IViewContainerDescriptor,
} from "../../../common/views.js";
import {
  ViewContainerLocation,
} from "../../../common/views.js";
import type {
  IViewDescriptorService,
} from "../../../services/views/common/viewDescriptorService.js";
import {
  CompositeBarActionViewItem,
} from "./compositeBarActionViewItem.js";

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
 * Selector for the Composites registered at one workbench location.
 *
 * A CompositeBar owns selection presentation only. Its containing Part owns
 * Composite construction, activation, visibility, and persisted state.
 */
export class CompositeBar extends DisposableOwner {
  readonly element: HTMLElement;
  readonly #viewDescriptorService: IViewDescriptorService;
  readonly #location: ViewContainerLocation;
  readonly #actionbar: ActionBar;
  readonly #items = new Map<string, CompositeBarActionViewItem>();
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
    this.#actionbar = this.own(new ActionBar({
      ownerDocument: options.ownerDocument,
      ariaRole: "tablist",
      ariaLabel: options.ariaLabel,
      actionViewItemProvider: (action) => {
        const item = new CompositeBarActionViewItem(action);
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

  get activeCompositeId(): string | undefined {
    return this.#activeCompositeId;
  }

  setActiveComposite(compositeId: string): void {
    if (!this.#items.has(compositeId)) {
      throw new Error(
        `Composite Bar item is not available: ${compositeId}`,
      );
    }
    if (this.#activeCompositeId === compositeId) return;
    this.#activeCompositeId = compositeId;
    this.#updateCheckedState();
  }

  #render(): void {
    this.#items.clear();
    const containers = this.#viewDescriptorService.getViewContainers(
      this.#location,
    );
    this.#actionbar.setActions(
      containers.map((container) => this.#createAction(container)),
    );
    for (const container of containers) {
      if (!this.#items.has(container.id)) {
        throw new Error(
          `Composite Bar action did not render: ${container.id}`,
        );
      }
    }
    if (
      this.#activeCompositeId !== undefined &&
      !this.#items.has(this.#activeCompositeId)
    ) {
      this.#activeCompositeId = undefined;
    }
    this.#updateCheckedState();
  }

  #createAction(container: IViewContainerDescriptor): IAction {
    return {
      id: container.id,
      label: container.title,
      tooltip: container.title,
      icon: container.icon,
      enabled: true,
      checked: container.id === this.#activeCompositeId,
      run: () => {
        if (this.#activeCompositeId === container.id) return;
        this.#onDidSelectComposite.fire({
          compositeId: container.id,
        });
      },
    };
  }

  #updateCheckedState(): void {
    for (const [compositeId, item] of this.#items) {
      item.setActive(compositeId === this.#activeCompositeId);
    }
  }
}
