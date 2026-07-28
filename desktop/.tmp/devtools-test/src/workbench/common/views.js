import { Emitter } from "../../base/common/event.js";
import { toDisposable, } from "../../base/common/lifecycle.js";
/** Workbench region capable of hosting registered view containers. */
export var ViewContainerLocation;
(function (ViewContainerLocation) {
    ViewContainerLocation["Sidebar"] = "sidebar";
    ViewContainerLocation["Panel"] = "panel";
    ViewContainerLocation["AuxiliaryBar"] = "auxiliarybar";
})(ViewContainerLocation || (ViewContainerLocation = {}));
/** Stable identifiers for the containers supplied by the core workbench. */
export const WorkbenchViewContainerId = Object.freeze({
    Sidebar: "zeta.sidebar",
    AuxiliaryBar: "zeta.auxiliary",
});
/**
 * Registry used by static contributions to declare view containers and views.
 *
 * Registrations are atomic: duplicate or invalid batches do not modify the
 * previous registry state. Disposing a container also removes its views.
 */
export class WorkbenchViewRegistry {
    #containers = new Map();
    #views = new Map();
    #onDidRegisterViewContainer = new Emitter();
    #onDidDeregisterViewContainer = new Emitter();
    #onDidRegisterViews = new Emitter();
    #onDidDeregisterViews = new Emitter();
    #nextOrder = 1;
    onDidRegisterViewContainer = this.#onDidRegisterViewContainer.event;
    onDidDeregisterViewContainer = this.#onDidDeregisterViewContainer.event;
    onDidRegisterViews = this.#onDidRegisterViews.event;
    onDidDeregisterViews = this.#onDidDeregisterViews.event;
    registerViewContainer(descriptor) {
        const registered = this.#addViewContainer(descriptor);
        return toDisposable(() => this.#removeViewContainer(registered));
    }
    /** Registers a process-lifetime container contribution. */
    registerStaticViewContainer(descriptor) {
        this.#addViewContainer(descriptor);
    }
    #addViewContainer(descriptor) {
        validateId(descriptor.id, "view container");
        validateTitle(descriptor.title, "View container");
        if (this.#containers.has(descriptor.id)) {
            throw new Error(`View container is already registered: ${descriptor.id}`);
        }
        if (descriptor.isDefault &&
            this.getViewContainers(descriptor.location).some((container) => container.isDefault)) {
            throw new Error(`Default view container is already registered for ${descriptor.location}`);
        }
        const registered = {
            descriptor: Object.freeze({ ...descriptor }),
            registrationOrder: this.#nextOrder++,
        };
        this.#containers.set(descriptor.id, registered);
        this.#onDidRegisterViewContainer.fire(registered.descriptor);
        return registered;
    }
    #removeViewContainer(registered) {
        const { descriptor } = registered;
        if (this.#containers.get(descriptor.id) !== registered)
            return;
        const views = this.getViews(descriptor.id);
        for (const view of views)
            this.#views.delete(view.id);
        if (views.length > 0) {
            this.#onDidDeregisterViews.fire({
                container: descriptor,
                views,
            });
        }
        this.#containers.delete(descriptor.id);
        this.#onDidDeregisterViewContainer.fire(descriptor);
    }
    registerViews(containerId, descriptors) {
        const registrations = this.#addViews(containerId, descriptors);
        return toDisposable(() => this.#removeViews(registrations));
    }
    /** Registers process-lifetime view contributions. */
    registerStaticViews(containerId, descriptors) {
        this.#addViews(containerId, descriptors);
    }
    #addViews(containerId, descriptors) {
        const container = this.#containers.get(containerId)?.descriptor;
        if (!container) {
            throw new Error(`Unknown view container: ${containerId}`);
        }
        const batchIds = new Set();
        for (const descriptor of descriptors) {
            validateId(descriptor.id, "view");
            validateTitle(descriptor.title, "View");
            if (batchIds.has(descriptor.id) ||
                this.#views.has(descriptor.id)) {
                throw new Error(`View is already registered: ${descriptor.id}`);
            }
            batchIds.add(descriptor.id);
        }
        const registrations = descriptors.map((descriptor) => ({
            descriptor: Object.freeze({ ...descriptor }),
            containerId,
            registrationOrder: this.#nextOrder++,
        }));
        for (const registration of registrations) {
            this.#views.set(registration.descriptor.id, registration);
        }
        const views = sortViews(registrations);
        if (views.length > 0) {
            this.#onDidRegisterViews.fire({ container, views });
        }
        return registrations;
    }
    #removeViews(registrations) {
        const removed = [];
        for (const registration of registrations) {
            if (this.#views.get(registration.descriptor.id) === registration) {
                this.#views.delete(registration.descriptor.id);
                removed.push(registration);
            }
        }
        if (removed.length === 0)
            return;
        const containerId = removed[0].containerId;
        const container = this.#containers.get(containerId)?.descriptor;
        if (!container)
            return;
        this.#onDidDeregisterViews.fire({
            container,
            views: sortViews(removed),
        });
    }
    getViewContainers(location) {
        return [...this.#containers.values()]
            .filter((registered) => location === undefined ||
            registered.descriptor.location === location)
            .sort(compareRegistered)
            .map((registered) => registered.descriptor);
    }
    getViewContainer(id) {
        return this.#containers.get(id)?.descriptor;
    }
    getDefaultViewContainer(location) {
        const containers = this.getViewContainers(location);
        return containers.find((container) => container.isDefault) ??
            containers[0];
    }
    getViews(containerId) {
        return sortViews([...this.#views.values()].filter((registered) => registered.containerId === containerId));
    }
    getView(id) {
        return this.#views.get(id)?.descriptor;
    }
    getViewContainerForView(viewId) {
        const containerId = this.#views.get(viewId)?.containerId;
        return containerId === undefined
            ? undefined
            : this.#containers.get(containerId)?.descriptor;
    }
}
/** Realm-wide view declarations populated by contribution modules. */
export const ViewsRegistry = new WorkbenchViewRegistry();
function sortViews(views) {
    return [...views]
        .sort(compareRegistered)
        .map((registered) => registered.descriptor);
}
function compareRegistered(left, right) {
    const leftOrder = left.descriptor.order ?? Number.MAX_SAFE_INTEGER;
    const rightOrder = right.descriptor.order ?? Number.MAX_SAFE_INTEGER;
    return leftOrder - rightOrder ||
        left.registrationOrder - right.registrationOrder;
}
function validateId(id, kind) {
    if (!/^[A-Za-z][A-Za-z0-9._-]{0,127}$/.test(id)) {
        throw new TypeError(`Invalid ${kind} ID: ${id}`);
    }
}
function validateTitle(title, kind) {
    if (!title.trim())
        throw new TypeError(`${kind} title must not be empty`);
}
