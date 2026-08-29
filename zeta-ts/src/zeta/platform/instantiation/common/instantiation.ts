import {
	Disposable,
	type IDisposable,
} from "../../../base/common/lifecycle.js";

/** A typed key for a service available while a command is executing. */
export type ServiceIdentifier<T> = symbol & {
	readonly __serviceType?: T;
};

/** Constructor accepted by synchronous instance descriptors. */
export type Constructor<T> = new (...args: any[]) => T;

/** Creates a stable typed service key. Export and reuse the returned value. */
export function createServiceIdentifier<T>(id: string): ServiceIdentifier<T> {
	return Symbol(id) as ServiceIdentifier<T>;
}

/** Provides command handlers with the services of the active application. */
export interface ServicesAccessor {
	get<T>(id: ServiceIdentifier<T>): T;
	getOptional<T>(id: ServiceIdentifier<T>): T | undefined;
}

/** Controls when a registered singleton is constructed. */
export enum InstantiationType {
	Eager = "eager",
	Delayed = "delayed",
}

/** Options for registering a singleton factory. */
export interface SingletonRegistrationOptions {
	readonly instantiation?: InstantiationType;
}

/** Factory used by the service container. */
export type ServiceFactory<T> = (accessor: ServicesAccessor) => T;

/** Options that describe arguments owned by a synchronous contribution. */
export interface SyncDescriptorOptions {
	readonly staticArguments?: readonly unknown[];
	readonly serviceDependencies?: readonly ServiceIdentifier<unknown>[];
}

/**
 * Describes how the instantiation service constructs a contributed object.
 *
 * Static arguments are placed before call-site arguments. Resolved services
 * are appended last, in the declared order.
 */
export class SyncDescriptor<T> {
	readonly staticArguments: readonly unknown[];
	readonly serviceDependencies: readonly ServiceIdentifier<unknown>[];

	constructor(
		readonly ctor: Constructor<T>,
		options: SyncDescriptorOptions = {},
	) {
		this.staticArguments = Object.freeze([
			...(options.staticArguments ?? []),
		]);
		this.serviceDependencies = Object.freeze([
			...(options.serviceDependencies ?? []),
		]);
	}
}

export class ServiceCollection {
	private readonly entriesById = new Map<ServiceIdentifier<unknown>, unknown>();

	constructor(...entries: readonly (readonly [ServiceIdentifier<unknown>, unknown])[]) {
		for (const [id, value] of entries) this.set(id, value);
	}

	set<T>(id: ServiceIdentifier<T>, value: T): T | undefined {
		const previous = this.entriesById.get(id) as T | undefined;
		this.entriesById.set(id, value);
		return previous;
	}

	has<T>(id: ServiceIdentifier<T>): boolean { return this.entriesById.has(id); }
	get<T>(id: ServiceIdentifier<T>): T | undefined { return this.entriesById.get(id) as T | undefined; }
	entries(): IterableIterator<[ServiceIdentifier<unknown>, unknown]> { return this.entriesById.entries(); }
}

export interface SingletonServiceDescriptor<T> {
	readonly id: ServiceIdentifier<T>;
	readonly factory: ServiceFactory<T>;
	readonly instantiation: InstantiationType;
}

const singletonServiceDescriptors = new Map<ServiceIdentifier<unknown>, SingletonServiceDescriptor<unknown>>();

export function registerSingleton<T>(id: ServiceIdentifier<T>, factory: ServiceFactory<T>, instantiation = InstantiationType.Delayed): SingletonServiceDescriptor<T> {
	if (singletonServiceDescriptors.has(id)) throw new Error(`Singleton service '${serviceName(id)}' is already registered`);
	const descriptor = Object.freeze({ id, factory, instantiation });
	singletonServiceDescriptors.set(id, descriptor as SingletonServiceDescriptor<unknown>);
	return descriptor;
}

export function getSingletonServiceDescriptors(): readonly SingletonServiceDescriptor<unknown>[] {
	return Object.freeze([...singletonServiceDescriptors.values()]);
}

/** The service container used by commands, contributions, and views. */
export interface IInstantiationService extends ServicesAccessor {
	createInstance<T>(
		descriptor: SyncDescriptor<T>,
		...dynamicArguments: unknown[]
	): T;

	createChild(): ServiceContainer;

	invokeFunction<R, TArguments extends unknown[]>(
		fn: (
			accessor: ServicesAccessor,
			...args: TArguments
		) => R,
		...args: TArguments
	): R;
}

export const IInstantiationService =
	createServiceIdentifier<IInstantiationService>("instantiationService");

interface InstanceRegistration<T> {
	readonly kind: "instance";
	readonly value: T;
}

interface FactoryRegistration<T> {
	readonly kind: "singleton" | "transient";
	readonly factory: ServiceFactory<T>;
	value: T | typeof UNINITIALIZED;
}

type ServiceRegistration<T> =
	| InstanceRegistration<T>
	| FactoryRegistration<T>;

const UNINITIALIZED = Symbol("service.uninitialized");

/**
 * Resolves services for one application scope.
 *
 * A child container inherits parent registrations and may override them for
 * its own scope. Singleton factories are lazy by default and disposable
 * values created by this container are owned by the container.
 */
export class ServiceContainer extends Disposable implements IInstantiationService {
	private readonly registrations = new Map<
		ServiceIdentifier<unknown>,
		ServiceRegistration<unknown>
	>();
	private readonly resolving: ServiceIdentifier<unknown>[] = [];

	constructor(private readonly parent?: ServiceContainer) {
		super();
		this.registerInstance(IInstantiationService, this);
	}

	registerInstance<T>(id: ServiceIdentifier<T>, value: T): void {
		this.assertNotDisposed();
		this.assertCanRegister(id);
		this.registrations.set(id, { kind: "instance", value });
	}

	registerSingleton<T>(
		id: ServiceIdentifier<T>,
		factory: ServiceFactory<T>,
		options: SingletonRegistrationOptions = {},
	): void {
		this.assertNotDisposed();
		this.assertCanRegister(id);
		this.registrations.set(id, {
			kind: "singleton",
			factory,
			value: UNINITIALIZED,
		});
		if (options.instantiation === InstantiationType.Eager) this.get(id);
	}

	registerTransient<T>(
		id: ServiceIdentifier<T>,
		factory: ServiceFactory<T>,
	): void {
		this.assertNotDisposed();
		this.assertCanRegister(id);
		this.registrations.set(id, {
			kind: "transient",
			factory,
			value: UNINITIALIZED,
		});
	}

	registerCollection(collection: ServiceCollection): void {
		for (const [id, value] of collection.entries()) this.registerInstance(id, value);
	}

	registerSingletonDescriptor<T>(descriptor: SingletonServiceDescriptor<T>): void {
		this.registerSingleton(descriptor.id, descriptor.factory, { instantiation: descriptor.instantiation });
	}

	has<T>(id: ServiceIdentifier<T>): boolean {
		return this.registrations.has(id) || this.parent?.has(id) === true;
	}

	get<T>(id: ServiceIdentifier<T>): T {
		this.assertNotDisposed();
		const registration = this.registrations.get(id);
		if (registration) return this.resolveRegistration(id, registration) as T;
		if (this.parent) return this.parent.get(id);
		throw new Error(`Unknown service: ${serviceName(id)}`);
	}

	getOptional<T>(id: ServiceIdentifier<T>): T | undefined {
		this.assertNotDisposed();
		const registration = this.registrations.get(id);
		if (registration) return this.resolveRegistration(id, registration) as T;
		return this.parent?.getOptional(id);
	}

	createChild(): ServiceContainer {
		this.assertNotDisposed();
		return new ServiceContainer(this);
	}

	createInstance<T>(
		descriptor: SyncDescriptor<T>,
		...dynamicArguments: unknown[]
	): T {
		this.assertNotDisposed();
		const serviceArguments = descriptor.serviceDependencies.map(
			(id) => this.get(id),
		);
		return Reflect.construct(descriptor.ctor, [
			...descriptor.staticArguments,
			...dynamicArguments,
			...serviceArguments,
		]) as T;
	}

	invokeFunction<R, TArguments extends unknown[]>(
		fn: (
			accessor: ServicesAccessor,
			...args: TArguments
		) => R,
		...args: TArguments
	): R {
		this.assertNotDisposed();
		let active = true;
		const accessor: ServicesAccessor = {
			get: <T>(id: ServiceIdentifier<T>): T => {
				if (!active) throw new ReferenceError("Service accessor is only valid during invocation");
				return this.get(id);
			},
			getOptional: <T>(id: ServiceIdentifier<T>): T | undefined => {
				if (!active) throw new ReferenceError("Service accessor is only valid during invocation");
				return this.getOptional(id);
			},
		};
		try {
			return fn(accessor, ...args);
		} finally {
			active = false;
		}
	}

	protected override disposeCore(): void {
		this.registrations.clear();
		this.resolving.length = 0;
		super.disposeCore();
	}

	private assertCanRegister<T>(id: ServiceIdentifier<T>): void {
		if (this.registrations.has(id)) {
			throw new Error(`Service '${serviceName(id)}' is already registered in this scope`);
		}
	}

	private resolveRegistration<T>(
		id: ServiceIdentifier<T>,
		registration: ServiceRegistration<T>,
	): T {
		if (registration.kind === "instance") return registration.value;
		if (registration.kind === "singleton" && registration.value !== UNINITIALIZED) {
			return registration.value;
		}
		this.beginResolving(id);
		try {
			const value = registration.factory(this);
			if (registration.kind === "singleton") {
				registration.value = value;
				if (isDisposable(value)) this._register(value);
			}
			return value;
		} finally {
			this.resolving.pop();
		}
	}

	private beginResolving(id: ServiceIdentifier<unknown>): void {
		const cycleStart = this.resolving.indexOf(id);
		if (cycleStart >= 0) {
			const cycle = [...this.resolving.slice(cycleStart), id]
				.map(serviceName)
				.join(" -> ");
			throw new Error(`Cyclic service dependency: ${cycle}`);
		}
		this.resolving.push(id);
	}
}

function isDisposable(value: unknown): value is IDisposable {
	return typeof value === "object"
		&& value !== null
		&& typeof (value as IDisposable).dispose === "function"
		&& typeof (value as IDisposable)[Symbol.dispose] === "function";
}

function serviceName(id: ServiceIdentifier<unknown>): string {
	return id.description ?? String(id);
}
