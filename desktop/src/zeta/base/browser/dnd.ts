import { DisposableOwner } from "../common/lifecycle.js";
import { addDisposableListener } from "./dom.js";

export const DataTransfers = {
  Text: "text/plain",
  UriList: "text/uri-list",
  Files: "Files",
} as const;

export interface DragAndDropObserverCallbacks {
  readonly onDragEnter?: (event: DragEvent) => void;
  readonly onDragOver?: (event: DragEvent) => void;
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

  constructor(
    readonly element: HTMLElement,
    callbacks: DragAndDropObserverCallbacks,
  ) {
    super();
    this.own(addDisposableListener(element, "dragenter", (event: DragEvent) => {
      this.dragDepth++;
      if (this.dragDepth === 1) callbacks.onDragEnter?.(event);
    }));
    this.own(addDisposableListener(element, "dragover", (event: DragEvent) =>
      callbacks.onDragOver?.(event),
    ));
    this.own(addDisposableListener(element, "dragleave", (event: DragEvent) => {
      this.dragDepth = Math.max(0, this.dragDepth - 1);
      if (this.dragDepth === 0) callbacks.onDragLeave?.(event);
    }));
    this.own(addDisposableListener(element, "drop", (event: DragEvent) => {
      this.dragDepth = 0;
      callbacks.onDrop?.(event);
    }));
    this.own(addDisposableListener(element, "dragend", (event: DragEvent) => {
      this.dragDepth = 0;
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
