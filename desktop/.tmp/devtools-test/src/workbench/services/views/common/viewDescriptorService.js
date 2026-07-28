import { Emitter } from "../../../../base/common/event.js";
import { DisposableOwner, } from "../../../../base/common/lifecycle.js";
import { createServiceIdentifier, } from "../../../../platform/instantiation/common/instantiation.js";
import { getVisibleViewContextKey, } from "../../../common/contextkeys.js";
import { ViewsRegistry, } from "../../../common/views.js";
export const IViewDescriptorService = createServiceIdentifier("viewDescriptorService");
/**
 * Owns the runtime models for all view containers in one workbench window.
 */
export class ViewDescriptorService extends DisposableOwner {
    #contextKeyService;
    #registry;
    #models = new Map();
    #onDidChangeViewContainers = this.own(new Emitter());
    onDidChangeViewContainers = this.#onDidChangeViewContainers.event;
    constructor(options) {
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
    getViewContainers(location) {
        return this.#registry.getViewContainers(location);
    }
    getDefaultViewContainer(location) {
        return this.#registry.getDefaultViewContainer(location);
    }
    getViewContainerModel(containerId) {
        const model = this.#models.get(containerId);
        if (!model)
            throw new Error(`Unknown view container: ${containerId}`);
        return model;
    }
    #addContainer(container) {
        if (this.#models.has(container.id))
            return;
        this.#models.set(container.id, this.own(new ViewContainerModel(container, this.#registry, this.#contextKeyService)));
    }
    #removeContainer(container) {
        const model = this.#models.get(container.id);
        if (!model)
            return;
        this.#models.delete(container.id);
        model.dispose();
    }
}
class ViewContainerModel extends DisposableOwner {
    viewContainer;
    #registry;
    #contextKeyService;
    #visibility = new Map();
    #visibilityContextKeys = new Map();
    #onDidChangeAllViewDescriptors = this.own(new Emitter());
    #onDidChangeActiveViewDescriptors = this.own(new Emitter());
    #onDidChangeVisibleViewDescriptors = this.own(new Emitter());
    #allViewDescriptors = [];
    #activeViewDescriptors = [];
    #visibleViewDescriptors = [];
    onDidChangeAllViewDescriptors = this.#onDidChangeAllViewDescriptors.event;
    onDidChangeActiveViewDescriptors = this.#onDidChangeActiveViewDescriptors.event;
    onDidChangeVisibleViewDescriptors = this.#onDidChangeVisibleViewDescriptors.event;
    constructor(viewContainer, registry, contextKeyService) {
        super();
        this.viewContainer = viewContainer;
        this.#registry = registry;
        this.#contextKeyService = contextKeyService;
        this.own(this.#registry.onDidRegisterViews((event) => {
            if (event.container.id === this.viewContainer.id)
                this.#recompute();
        }));
        this.own(this.#registry.onDidDeregisterViews((event) => {
            if (event.container.id === this.viewContainer.id)
                this.#recompute();
        }));
        this.own(this.#contextKeyService.onDidChangeContext((event) => {
            if (this.#affectsActiveViews(event))
                this.#recompute();
        }));
        this.defer(() => {
            for (const key of this.#visibilityContextKeys.values())
                key.reset();
            this.#visibilityContextKeys.clear();
            this.#visibility.clear();
        });
        this.#recompute();
    }
    get allViewDescriptors() {
        return this.#allViewDescriptors;
    }
    get activeViewDescriptors() {
        return this.#activeViewDescriptors;
    }
    get visibleViewDescriptors() {
        return this.#visibleViewDescriptors;
    }
    isVisible(viewId) {
        return this.#visibleViewDescriptors.some((view) => view.id === viewId);
    }
    setVisible(viewId, visible) {
        const descriptor = this.#allViewDescriptors.find((view) => view.id === viewId);
        if (!descriptor)
            throw new Error(`Unknown view: ${viewId}`);
        if (descriptor.canToggleVisibility === false) {
            throw new Error(`View visibility cannot be changed: ${viewId}`);
        }
        if (this.#preferredVisibility(descriptor) === visible)
            return;
        this.#visibility.set(viewId, visible);
        this.#recompute();
    }
    #affectsActiveViews(event) {
        return this.#allViewDescriptors.some((view) => view.when !== undefined && event.affectsSome(view.when.keys()));
    }
    #recompute() {
        const previousAll = this.#allViewDescriptors;
        const previousActive = this.#activeViewDescriptors;
        const previousVisible = this.#visibleViewDescriptors;
        const all = this.#registry.getViews(this.viewContainer.id);
        const active = all.filter((view) => this.#contextKeyService.contextMatchesRules(view.when));
        const visible = active.filter((view) => this.#preferredVisibility(view));
        this.#allViewDescriptors = all;
        this.#activeViewDescriptors = active;
        this.#visibleViewDescriptors = visible;
        this.#updateVisibilityContextKeys(all, visible);
        fireDescriptorChanges(this.#onDidChangeAllViewDescriptors, previousAll, all);
        fireDescriptorChanges(this.#onDidChangeActiveViewDescriptors, previousActive, active);
        fireDescriptorChanges(this.#onDidChangeVisibleViewDescriptors, previousVisible, visible);
    }
    #preferredVisibility(descriptor) {
        return this.#visibility.get(descriptor.id) ??
            descriptor.hideByDefault !== true;
    }
    #updateVisibilityContextKeys(all, visible) {
        const allIds = new Set(all.map((view) => view.id));
        const visibleIds = new Set(visible.map((view) => view.id));
        for (const [viewId, key] of this.#visibilityContextKeys) {
            if (allIds.has(viewId))
                continue;
            key.reset();
            this.#visibilityContextKeys.delete(viewId);
            this.#visibility.delete(viewId);
        }
        for (const descriptor of all) {
            let key = this.#visibilityContextKeys.get(descriptor.id);
            if (!key) {
                key = this.#contextKeyService.createKey(getVisibleViewContextKey(descriptor.id), false);
                this.#visibilityContextKeys.set(descriptor.id, key);
            }
            key.set(visibleIds.has(descriptor.id));
        }
    }
}
function fireDescriptorChanges(emitter, previous, next) {
    const previousIds = new Set(previous.map((view) => view.id));
    const nextIds = new Set(next.map((view) => view.id));
    const added = next.filter((view) => !previousIds.has(view.id));
    const removed = previous.filter((view) => !nextIds.has(view.id));
    if (added.length === 0 && removed.length === 0)
        return;
    emitter.fire({ added, removed });
}
