import {
  type IDisposableTracker,
  registerDisposableTracker,
  type TrackableDisposable,
} from "./lifecycle.js";

export interface DisposableLeak {
  readonly disposable: TrackableDisposable;
  readonly label: string;
  readonly ownerLabel?: string;
  readonly createdAt?: string;
}

interface DisposableRecord {
  readonly disposable: TrackableDisposable;
  readonly label: string;
  readonly createdAt?: string;
  readonly children: Set<TrackableDisposable>;
  owner?: TrackableDisposable;
}

/**
 * Development-time ownership graph for diagnosing leaked or multiply-owned
 * disposables.
 *
 * The tracker intentionally retains live disposables strongly. Install it only
 * in development or tests and call `assertNoLeaks` at a well-defined scope
 * boundary.
 */
export class DisposableTracker implements IDisposableTracker {
  private readonly records = new Map<TrackableDisposable, DisposableRecord>();
  private readonly disposed = new WeakSet<object>();

  trackDisposable(
    disposable: TrackableDisposable,
    label = disposableLabel(disposable),
  ): void {
    if (this.disposed.has(disposable)) {
      throw new ReferenceError(`Cannot track disposed disposable: ${label}`);
    }
    if (this.records.has(disposable)) return;
    this.records.set(disposable, {
      disposable,
      label,
      createdAt: captureCreationStack(),
      children: new Set(),
    });
  }

  validateDisposableOwner(
    disposable: TrackableDisposable,
    owner: TrackableDisposable,
  ): void {
    if (disposable === owner) {
      throw new Error("A disposable cannot own itself");
    }
    if (this.disposed.has(disposable)) {
      throw new ReferenceError(
        `Cannot own disposed disposable: ${disposableLabel(disposable)}`,
      );
    }
    const existingOwner = this.records.get(disposable)?.owner;
    if (existingOwner && existingOwner !== owner) {
      throw new Error(
        `${this.label(disposable)} already belongs to ${this.label(existingOwner)}`,
      );
    }
    for (
      let ancestor: TrackableDisposable | undefined = owner;
      ancestor;
      ancestor = this.records.get(ancestor)?.owner
    ) {
      if (ancestor === disposable) {
        throw new Error("Disposable ownership cannot contain a cycle");
      }
    }
  }

  setDisposableOwner(
    disposable: TrackableDisposable,
    owner: TrackableDisposable,
  ): void {
    this.validateDisposableOwner(disposable, owner);
    this.trackDisposable(owner);
    this.trackDisposable(disposable);
    const record = this.records.get(disposable);
    if (!record || record.owner === owner) return;
    record.owner = owner;
    this.records.get(owner)?.children.add(disposable);
  }

  markAsDisposed(disposable: TrackableDisposable): void {
    const record = this.records.get(disposable);
    if (record) {
      for (const child of [...record.children]) {
        this.markAsDisposed(child);
      }
      if (record.owner) {
        this.records.get(record.owner)?.children.delete(disposable);
      }
      this.records.delete(disposable);
    }
    this.disposed.add(disposable);
  }

  leaks(): readonly DisposableLeak[] {
    return [...this.records.values()].map((record) => ({
      disposable: record.disposable,
      label: record.label,
      ownerLabel: record.owner ? this.label(record.owner) : undefined,
      createdAt: record.createdAt,
    }));
  }

  assertNoLeaks(): void {
    const leaks = this.leaks();
    if (leaks.length === 0) return;
    const details = leaks.map((leak) => {
      const ownership = leak.ownerLabel
        ? ` owned by ${leak.ownerLabel}`
        : " without an owner";
      return `${leak.label}${ownership}${leak.createdAt ? `\n${leak.createdAt}` : ""}`;
    });
    throw new Error(
      `Detected ${leaks.length} undisposed disposable(s):\n${details.join("\n")}`,
    );
  }

  private label(disposable: TrackableDisposable): string {
    return this.records.get(disposable)?.label ??
      disposableLabel(disposable);
  }
}

/**
 * Installs the development tracker for the current JavaScript realm.
 */
export function installDisposableTracker(
  tracker: DisposableTracker,
): Disposable {
  return registerDisposableTracker(tracker);
}

function disposableLabel(disposable: TrackableDisposable): string {
  const constructor = (disposable as object).constructor;
  return typeof constructor === "function" && constructor.name
    ? constructor.name
    : "Disposable";
}

function captureCreationStack(): string | undefined {
  return new Error().stack
    ?.split("\n")
    .slice(3)
    .join("\n");
}
