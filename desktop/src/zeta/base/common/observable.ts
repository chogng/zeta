import type { Event } from "./event.js";
import { toDisposable } from "./lifecycle.js";

/** Reads an observable while allowing a reactive owner to record the dependency. */
export interface IReader {
  readObservable<T>(observable: IObservable<T>): T;
}

/** Minimal common observable contract used by development-time reload helpers. */
export interface IObservable<T> {
  get(): T;
  read(reader: IReader | undefined): T;
  readonly onDidChange: Event<T>;
}

/** Observable whose current value can be replaced synchronously. */
export interface ISettableObservable<T> extends IObservable<T> {
  set(value: T, transaction?: undefined): void;
}

/** Creates an observable that never changes. */
export function constObservable<T>(value: T): IObservable<T> {
  const observable: IObservable<T> = {
    get: () => value,
    read: reader => reader ? reader.readObservable(observable) : value,
    onDidChange: () => toDisposable(() => {}),
  };
  return observable;
}

/** Creates a named mutable observable value. */
export function observableValue<T>(_nameOrOwner: string | object, initialValue: T): ISettableObservable<T> {
  return new ObservableValue(initialValue);
}

/** Converts an event into an observable invalidation signal. */
export function observableSignalFromEvent(_nameOrOwner: string | object, event: Event<unknown>): IObservable<void> {
  const signal: IObservable<void> = {
    get: () => undefined,
    read: reader => reader ? reader.readObservable(signal) : undefined,
    onDidChange: listener => event(() => listener(undefined)),
  };
  return signal;
}

class ObservableValue<T> implements ISettableObservable<T> {
  private readonly listeners = new Set<(value: T) => void>();
  private value: T;

  readonly onDidChange: Event<T> = listener => {
    this.listeners.add(listener);
    return toDisposable(() => this.listeners.delete(listener));
  };

  constructor(initialValue: T) {
    this.value = initialValue;
  }

  get(): T {
    return this.value;
  }

  read(reader: IReader | undefined): T {
    return reader ? reader.readObservable(this) : this.value;
  }

  set(value: T, _transaction?: undefined): void {
    if (Object.is(this.value, value)) return;
    this.value = value;
    for (const listener of [...this.listeners]) listener(value);
  }
}
