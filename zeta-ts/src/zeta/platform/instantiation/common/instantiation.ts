/** A typed key for a service available while a command is executing. */
export type ServiceIdentifier<T> = symbol & {
  readonly __serviceType?: T;
};

/** Constructor accepted by synchronous instance descriptors. */
export type Constructor<T> = new (...args: any[]) => T;

/** Creates a stable typed service key. Export and reuse the returned value. */
export function createServiceIdentifier<T>(
  id: string,
): ServiceIdentifier<T> {
  return Symbol(id) as ServiceIdentifier<T>;
}

/** Provides command handlers with the services of the active application. */
export interface ServicesAccessor {
  get<T>(id: ServiceIdentifier<T>): T;
}

/** Options that describe arguments owned by a synchronous contribution. */
export interface SyncDescriptorOptions {
  readonly staticArguments?: readonly unknown[];
  readonly serviceDependencies?:
    readonly ServiceIdentifier<unknown>[];
}

/**
 * Describes how the instantiation service constructs a contributed object.
 *
 * Static arguments are placed before call-site arguments. Resolved services
 * are appended last, in the declared order.
 */
export class SyncDescriptor<T> {
  readonly staticArguments: readonly unknown[];
  readonly serviceDependencies:
    readonly ServiceIdentifier<unknown>[];

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

/** Creates contributed objects against one window's service collection. */
export interface IInstantiationService extends ServicesAccessor {
  createInstance<T>(
    descriptor: SyncDescriptor<T>,
    ...dynamicArguments: unknown[]
  ): T;

  invokeFunction<R, TArguments extends unknown[]>(
    fn: (
      accessor: ServicesAccessor,
      ...args: TArguments
    ) => R,
    ...args: TArguments
  ): R;
}

export const IInstantiationService =
  createServiceIdentifier<IInstantiationService>(
    "instantiationService",
  );

/** A minimal service collection used as the command execution accessor. */
export class ServiceCollection implements ServicesAccessor {
  private readonly services = new Map<ServiceIdentifier<unknown>, unknown>();

  set<T>(id: ServiceIdentifier<T>, service: T): void {
    this.services.set(id, service);
  }

  has<T>(id: ServiceIdentifier<T>): boolean {
    return this.services.has(id);
  }

  get<T>(id: ServiceIdentifier<T>): T {
    if (!this.services.has(id)) {
      throw new Error(`Unknown service: ${id.description ?? String(id)}`);
    }
    return this.services.get(id) as T;
  }

  getOptional<T>(id: ServiceIdentifier<T>): T | undefined {
    return this.services.get(id) as T | undefined;
  }
}

/**
 * Resolves explicitly declared constructor dependencies from a service
 * collection. Constructor metadata stays explicit so platform consumers do
 * not depend on TypeScript decorator emit.
 */
export class InstantiationService implements IInstantiationService {
  private readonly services: ServiceCollection;

  constructor(services: ServiceCollection = new ServiceCollection()) {
    this.services = services;
    this.services.set(IInstantiationService, this);
  }

  get<T>(id: ServiceIdentifier<T>): T {
    return this.services.get(id);
  }

  createInstance<T>(
    descriptor: SyncDescriptor<T>,
    ...dynamicArguments: unknown[]
  ): T {
    const serviceArguments = descriptor.serviceDependencies.map(
      (id) => this.services.get(id),
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
    let active = true;
    const accessor: ServicesAccessor = {
      get: <T>(id: ServiceIdentifier<T>): T => {
        if (!active) {
          throw new ReferenceError(
            "Service accessor is only valid during invocation",
          );
        }
        return this.services.get(id);
      },
    };
    try {
      return fn(accessor, ...args);
    } finally {
      active = false;
    }
  }
}
