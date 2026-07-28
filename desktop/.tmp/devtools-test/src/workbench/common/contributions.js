import { DisposableOwner, toDisposable, } from "../../base/common/lifecycle.js";
/** Startup point at which a workbench contribution is instantiated. */
export var WorkbenchPhase;
(function (WorkbenchPhase) {
    WorkbenchPhase[WorkbenchPhase["BlockStartup"] = 1] = "BlockStartup";
    WorkbenchPhase[WorkbenchPhase["BlockRestore"] = 2] = "BlockRestore";
    WorkbenchPhase[WorkbenchPhase["AfterRestored"] = 3] = "AfterRestored";
    WorkbenchPhase[WorkbenchPhase["Eventually"] = 4] = "Eventually";
})(WorkbenchPhase || (WorkbenchPhase = {}));
/**
 * Realm-wide declarations used to create a separate contribution host for
 * each workbench window.
 */
export class WorkbenchContributionRegistry {
    #registrations = new Map();
    #nextOrder = 1;
    register(id, phase, factory) {
        const registration = this.#add(id, phase, factory);
        return toDisposable(() => {
            if (this.#registrations.get(id) === registration) {
                this.#registrations.delete(id);
            }
        });
    }
    /** Registers a declaration that intentionally lives for the realm. */
    registerStatic(id, phase, factory) {
        this.#add(id, phase, factory);
    }
    #add(id, phase, factory) {
        validateContributionId(id);
        validateWorkbenchPhase(phase);
        if (this.#registrations.has(id)) {
            throw new Error(`Workbench contribution is already registered: ${id}`);
        }
        const registration = {
            id,
            phase,
            factory,
            order: this.#nextOrder++,
        };
        this.#registrations.set(id, registration);
        return registration;
    }
    /**
     * Creates a window host from the contributions registered at this moment.
     */
    createHost(accessor, onError = defaultErrorHandler) {
        return new WorkbenchContributionHost(accessor, [...this.#registrations.values()].sort((left, right) => left.order - right.order), onError);
    }
}
/**
 * Owns the contribution instances of one workbench window and advances their
 * startup phases monotonically.
 */
export class WorkbenchContributionHost extends DisposableOwner {
    #accessor;
    #registrations;
    #onError;
    #instantiated = new Set();
    #phase = 0;
    constructor(accessor, registrations, onError) {
        super();
        this.#accessor = accessor;
        this.#registrations = registrations;
        this.#onError = onError;
    }
    get phase() {
        return this.#phase === 0
            ? undefined
            : this.#phase;
    }
    /**
     * Instantiates every pending contribution through the requested phase.
     */
    advance(phase) {
        validateWorkbenchPhase(phase);
        if (phase < this.#phase) {
            throw new Error("Workbench contribution phases cannot move backwards");
        }
        if (phase === this.#phase)
            return;
        this.#phase = phase;
        for (const registration of this.#registrations) {
            if (registration.phase > phase ||
                this.#instantiated.has(registration.id)) {
                continue;
            }
            this.#instantiated.add(registration.id);
            try {
                this.own(registration.factory(this.#accessor));
            }
            catch (error) {
                this.#onError(error, registration.id);
            }
        }
    }
}
/** Realm-wide workbench contribution declarations. */
export const WorkbenchContributionsRegistry = new WorkbenchContributionRegistry();
/**
 * Registers a process-lifetime contribution declaration.
 *
 * Contribution modules should call this during module evaluation, before a
 * workbench host is created.
 */
export function registerWorkbenchContribution(id, phase, factory) {
    WorkbenchContributionsRegistry.registerStatic(id, phase, factory);
}
function validateContributionId(id) {
    if (!/^[A-Za-z][A-Za-z0-9._-]{0,127}$/.test(id)) {
        throw new TypeError(`Invalid workbench contribution ID: ${id}`);
    }
}
function validateWorkbenchPhase(phase) {
    if (!Number.isInteger(phase) ||
        phase < WorkbenchPhase.BlockStartup ||
        phase > WorkbenchPhase.Eventually) {
        throw new TypeError(`Invalid workbench phase: ${phase}`);
    }
}
function defaultErrorHandler(error, contributionId) {
    console.error(`Unable to create workbench contribution '${contributionId}'`, error);
}
