import { DisposableOwner } from "../common/lifecycle.js";
import { addDisposableListener } from "./dom.js";

export const DataTransfers = {
  Text: "text/plain",
  UriList: "text/uri-list",
  Files: "Files",
} as const;

export interface DragAndDropObserverCallbacks {
  readonly onDragStart?: (event: DragEvent) => void;
  readonly onDrag?: (event: DragEvent) => void;
  readonly onDragEnter?: (event: DragEvent) => void;
  readonly onDragOver?: (event: DragEvent, duration: number) => void;
  readonly onDragLeave?: (event: DragEvent) => void;
  readonly onDrop?: (event: DragEvent) => void;
  readonly onDragEnd?: (event: DragEvent) => void;
}

/**
 * Normalizes nested dragenter/dragleave events for one drop target and owns
 * all native listener registrations.
 */
export class DragAndDropObserver extends DisposableOwner {
  private dragDepth = 0;
  private dragStartedAt: number | undefined;

  constructor(
    readonly element: HTMLElement,
    callbacks: DragAndDropObserverCallbacks,
  ) {
    super();
    if (callbacks.onDragStart) {
      this.own(addDisposableListener(element, "dragstart", (event: DragEvent) => {
        callbacks.onDragStart?.(event);
      }));
    }
    if (callbacks.onDrag) {
      this.own(addDisposableListener(element, "drag", (event: DragEvent) => {
        callbacks.onDrag?.(event);
      }));
    }
    this.own(addDisposableListener(element, "dragenter", (event: DragEvent) => {
      this.dragDepth++;
      if (this.dragDepth !== 1) return;
      this.dragStartedAt = event.timeStamp;
      callbacks.onDragEnter?.(event);
    }));
    this.own(addDisposableListener(element, "dragover", (event: DragEvent) => {
      event.preventDefault();
      this.dragStartedAt ??= event.timeStamp;
      callbacks.onDragOver?.(event, event.timeStamp - this.dragStartedAt);
    }));
    this.own(addDisposableListener(element, "dragleave", (event: DragEvent) => {
      this.dragDepth = Math.max(0, this.dragDepth - 1);
      if (this.dragDepth !== 0) return;
      this.dragStartedAt = undefined;
      callbacks.onDragLeave?.(event);
    }));
    this.own(addDisposableListener(element, "drop", (event: DragEvent) => {
      this.dragDepth = 0;
      this.dragStartedAt = undefined;
      callbacks.onDrop?.(event);
    }));
    this.own(addDisposableListener(element, "dragend", (event: DragEvent) => {
      this.dragDepth = 0;
      this.dragStartedAt = undefined;
      callbacks.onDragEnd?.(event);
    }));
  }
}

export function containsDragType(
  event: DragEvent,
  type: string,
): boolean {
  return [...(event.dataTransfer?.types ?? [])].some(
    (candidate) => candidate.toLowerCase() === type.toLowerCase(),
  );
}
