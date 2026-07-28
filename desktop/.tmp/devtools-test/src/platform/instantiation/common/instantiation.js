/** Creates a stable typed service key. Export and reuse the returned value. */
export function createServiceIdentifier(id) {
    return Symbol(id);
}
/**
 * Describes how the instantiation service constructs a contributed object.
 *
 * Static arguments are placed before call-site arguments. Resolved services
 * are appended last, in the declared order.
 */
export class SyncDescriptor {
    ctor;
    staticArguments;
    serviceDependencies;
    constructor(ctor, options = {}) {
        this.ctor = ctor;
        this.staticArguments = Object.freeze([
            ...(options.staticArguments ?? []),
        ]);
        this.serviceDependencies = Object.freeze([
            ...(options.serviceDependencies ?? []),
        ]);
    }
}
export const IInstantiationService = createServiceIdentifier("instantiationService");
/** A minimal service collection used as the command execution accessor. */
export class ServiceCollection {
    #services = new Map();
    set(id, service) {
        this.#services.set(id, service);
    }
    get(id) {
        if (!this.#services.has(id)) {
            throw new Error(`Unknown service: ${id.description ?? String(id)}`);
        }
        return this.#services.get(id);
    }
}
/**
 * Resolves explicitly declared constructor dependencies from a service
 * collection. Constructor metadata stays explicit so platform consumers do
 * not depend on TypeScript decorator emit.
 */
export class InstantiationService {
    #services;
    constructor(services = new ServiceCollection()) {
        this.#services = services;
        this.#services.set(IInstantiationService, this);
    }
    get(id) {
        return this.#services.get(id);
    }
    createInstance(descriptor, ...dynamicArguments) {
        const serviceArguments = descriptor.serviceDependencies.map((id) => this.#services.get(id));
        return Reflect.construct(descriptor.ctor, [
            ...descriptor.staticArguments,
            ...dynamicArguments,
            ...serviceArguments,
        ]);
    }
    invokeFunction(fn, ...args) {
        let active = true;
        const accessor = {
            get: (id) => {
                if (!active) {
                    throw new ReferenceError("Service accessor is only valid during invocation");
                }
                return this.#services.get(id);
            },
        };
        try {
            return fn(accessor, ...args);
        }
        finally {
            active = false;
        }
    }
}
