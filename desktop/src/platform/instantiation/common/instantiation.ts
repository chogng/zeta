/** A typed key for a service available while a command is executing. */
export type ServiceIdentifier<T> = symbol & {
  readonly __serviceType?: T;
};

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

/** A minimal service collection used as the command execution accessor. */
export class ServiceCollection implements ServicesAccessor {
  readonly #services = new Map<ServiceIdentifier<unknown>, unknown>();

  set<T>(id: ServiceIdentifier<T>, service: T): void {
    this.#services.set(id, service);
  }

  get<T>(id: ServiceIdentifier<T>): T {
    if (!this.#services.has(id)) {
      throw new Error(`Unknown service: ${id.description ?? String(id)}`);
    }
    return this.#services.get(id) as T;
  }
}
