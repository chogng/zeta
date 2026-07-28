import { Emitter, type Event } from "../../../../base/common/event.js";
import {
  DisposableOwner,
  type IDisposable,
} from "../../../../base/common/lifecycle.js";
import {
  type ContextKeyChangeEvent,
  type IContextKey,
  type IContextKeyService,
} from "../../../../platform/contextkey/common/contextkey.js";
import {
  createServiceIdentifier,
} from "../../../../platform/instantiation/common/instantiation.js";
import {
  getVisibleViewContextKey,
} from "../../../common/contextkeys.js";
import {
  type IViewContainerDescriptor,
  type IViewContainerModel,
  type IViewDescriptor,
  type IViewDescriptorsChangeEvent,
  ViewContainerLocation,
  type WorkbenchViewRegistry,
  ViewsRegistry,
} from "../../../common/views.js";

/** Change to the containers available in one workbench window. */
export interface IViewContainersChangeEvent {
  readonly added: readonly IViewContainerDescriptor[];
  readonly removed: readonly IViewContainerDescriptor[];
}

/**
 * Window-scoped access to context-aware view container models.
 *
 * Implementations project realm-wide static declarations into independent
 * per-window visibility state.
 */
export interface IViewDescriptorService {
  readonly onDidChangeViewContainers: Event<IViewContainersChangeEvent>;

  getViewContainers(
    location: ViewContainerLocation,
  ): readonly IViewContainerDescriptor[];
  getDefaultViewContainer(
    location: ViewContainerLocation,
  ): IViewContainerDescriptor | undefined;
  getViewContainerModel(
    containerId: string,
  ): IViewContainerModel;
}

export const IViewDescriptorService =
  createServiceIdentifier<IViewDescriptorService>(
    "viewDescriptorService",
  );

export interface ViewDescriptorServiceOptions {
  readonly contextKeyService: IContextKeyService;
  readonly registry?: WorkbenchViewRegistry;
}

/**
 * Owns the runtime models for all view containers in one workbench window.
 */
export class ViewDescriptorService
  extends DisposableOwner
  implements IViewDescriptorService {
  readonly #contextKeyService: IContextKeyService;
  readonly #registry: WorkbenchViewRegistry;
  readonly #models = new Map<string, ViewContainerModel>();
  readonly #onDidChangeViewContainers =
    this.own(new Emitter<IViewContainersChangeEvent>());

  readonly onDidChangeViewContainers =
    this.#onDidChangeViewContainers.event;

  constructor(options: ViewDescriptorServiceOptions) {
    super();
    this.#contextKeyService = options.contextKeyService;
    this.#registry = options.registry ?? ViewsRegistry;
    for (const container of this.#registry.getViewContainers()) {
      this.#addContainer(container);
    }
    this.own(this.#registry.onDidRegisterViewContainer((container) => {
      this.#addContainer(container);
      this.#onDidChangeViewContainers.fire({
        added: [container],
        removed: [],
      });
    }));
    this.own(this.#registry.onDidDeregisterViewContainer((container) => {
      this.#removeContainer(container);
      this.#onDidChangeViewContainers.fire({
        added: [],
        removed: [container],
      });
    }));
    this.defer(() => this.#models.clear());
  }

  getViewContainers(
    location: ViewContainerLocation,
  ): readonly IViewContainerDescriptor[] {
    return this.#registry.getViewContainers(location);
  }

  getDefaultViewContainer(
    location: ViewContainerLocation,
  ): IViewContainerDescriptor | undefined {
    return this.#registry.getDefaultViewContainer(location);
  }

  getViewContainerModel(containerId: string): IViewContainerModel {
    const model = this.#models.get(containerId);
    if (!model) throw new Error(`Unknown view container: ${containerId}`);
    return model;
  }

  #addContainer(container: IViewContainerDescriptor): void {
    if (this.#models.has(container.id)) return;
    this.#models.set(
      container.id,
      this.own(new ViewContainerModel(
        container,
        this.#registry,
        this.#contextKeyService,
      )),
    );
  }

  #removeContainer(container: IViewContainerDescriptor): void {
    const model = this.#models.get(container.id);
    if (!model) return;
    this.#models.delete(container.id);
    model.dispose();
  }
}

class ViewContainerModel
  extends DisposableOwner
  implements IViewContainerModel {
  readonly #registry: WorkbenchViewRegistry;
  readonly #contextKeyService: IContextKeyService;
  readonly #visibility = new Map<string, boolean>();
  readonly #visibilityContextKeys =
    new Map<string, IContextKey<boolean>>();
  readonly #onDidChangeAllViewDescriptors =
    this.own(new Emitter<IViewDescriptorsChangeEvent>());
  readonly #onDidChangeActiveViewDescriptors =
    this.own(new Emitter<IViewDescriptorsChangeEvent>());
  readonly #onDidChangeVisibleViewDescriptors =
    this.own(new Emitter<IViewDescriptorsChangeEvent>());
  #allViewDescriptors: readonly IViewDescriptor[] = [];
  #activeViewDescriptors: readonly IViewDescriptor[] = [];
  #visibleViewDescriptors: readonly IViewDescriptor[] = [];

  readonly onDidChangeAllViewDescriptors =
    this.#onDidChangeAllViewDescriptors.event;
  readonly onDidChangeActiveViewDescriptors =
    this.#onDidChangeActiveViewDescriptors.event;
  readonly onDidChangeVisibleViewDescriptors =
    this.#onDidChangeVisibleViewDescriptors.event;

  constructor(
    readonly viewContainer: IViewContainerDescriptor,
    registry: WorkbenchViewRegistry,
    contextKeyService: IContextKeyService,
  ) {
    super();
    this.#registry = registry;
    this.#contextKeyService = contextKeyService;
    this.own(this.#registry.onDidRegisterViews((event) => {
      if (event.container.id === this.viewContainer.id) this.#recompute();
    }));
    this.own(this.#registry.onDidDeregisterViews((event) => {
      if (event.container.id === this.viewContainer.id) this.#recompute();
    }));
    this.own(this.#contextKeyService.onDidChangeContext((event) => {
      if (this.#affectsActiveViews(event)) this.#recompute();
    }));
    this.defer(() => {
      for (const key of this.#visibilityContextKeys.values()) key.reset();
      this.#visibilityContextKeys.clear();
      this.#visibility.clear();
    });
    this.#recompute();
  }

  get allViewDescriptors(): readonly IViewDescriptor[] {
    return this.#allViewDescriptors;
  }

  get activeViewDescriptors(): readonly IViewDescriptor[] {
    return this.#activeViewDescriptors;
  }

  get visibleViewDescriptors(): readonly IViewDescriptor[] {
    return this.#visibleViewDescriptors;
  }

  isVisible(viewId: string): boolean {
    return this.#visibleViewDescriptors.some((view) => view.id === viewId);
  }

  setVisible(viewId: string, visible: boolean): void {
    const descriptor = this.#allViewDescriptors.find(
      (view) => view.id === viewId,
    );
    if (!descriptor) throw new Error(`Unknown view: ${viewId}`);
    if (descriptor.canToggleVisibility === false) {
      throw new Error(`View visibility cannot be changed: ${viewId}`);
    }
    if (this.#preferredVisibility(descriptor) === visible) return;
    this.#visibility.set(viewId, visible);
    this.#recompute();
  }

  #affectsActiveViews(event: ContextKeyChangeEvent): boolean {
    return this.#allViewDescriptors.some((view) =>
      view.when !== undefined && event.affectsSome(view.when.keys())
    );
  }

  #recompute(): void {
    const previousAll = this.#allViewDescriptors;
    const previousActive = this.#activeViewDescriptors;
    const previousVisible = this.#visibleViewDescriptors;
    const all = this.#registry.getViews(this.viewContainer.id);
    const active = all.filter((view) =>
      this.#contextKeyService.contextMatchesRules(view.when)
    );
    const visible = active.filter((view) =>
      this.#preferredVisibility(view)
    );

    this.#allViewDescriptors = all;
    this.#activeViewDescriptors = active;
    this.#visibleViewDescriptors = visible;
    this.#updateVisibilityContextKeys(all, visible);
    fireDescriptorChanges(
      this.#onDidChangeAllViewDescriptors,
      previousAll,
      all,
    );
    fireDescriptorChanges(
      this.#onDidChangeActiveViewDescriptors,
      previousActive,
      active,
    );
    fireDescriptorChanges(
      this.#onDidChangeVisibleViewDescriptors,
      previousVisible,
      visible,
    );
  }

  #preferredVisibility(descriptor: IViewDescriptor): boolean {
    return this.#visibility.get(descriptor.id) ??
      descriptor.hideByDefault !== true;
  }

  #updateVisibilityContextKeys(
    all: readonly IViewDescriptor[],
    visible: readonly IViewDescriptor[],
  ): void {
    const allIds = new Set(all.map((view) => view.id));
    const visibleIds = new Set(visible.map((view) => view.id));
    for (const [viewId, key] of this.#visibilityContextKeys) {
      if (allIds.has(viewId)) continue;
      key.reset();
      this.#visibilityContextKeys.delete(viewId);
      this.#visibility.delete(viewId);
    }
    for (const descriptor of all) {
      let key = this.#visibilityContextKeys.get(descriptor.id);
      if (!key) {
        key = this.#contextKeyService.createKey(
          getVisibleViewContextKey(descriptor.id),
          false,
        );
        this.#visibilityContextKeys.set(descriptor.id, key);
      }
      key.set(visibleIds.has(descriptor.id));
    }
  }
}

function fireDescriptorChanges(
  emitter: Emitter<IViewDescriptorsChangeEvent>,
  previous: readonly IViewDescriptor[],
  next: readonly IViewDescriptor[],
): void {
  const previousIds = new Set(previous.map((view) => view.id));
  const nextIds = new Set(next.map((view) => view.id));
  const added = next.filter((view) => !previousIds.has(view.id));
  const removed = previous.filter((view) => !nextIds.has(view.id));
  if (added.length === 0 && removed.length === 0) return;
  emitter.fire({ added, removed });
}
