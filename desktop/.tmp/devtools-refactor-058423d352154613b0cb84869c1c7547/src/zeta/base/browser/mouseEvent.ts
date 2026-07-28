import { stopEvent } from "./dom.js";
import { getWindow } from "./window.js";

/** Normalized mouse-event coordinates and buttons across browser windows. */
export class StandardMouseEvent {
  readonly target: EventTarget | null;
  readonly leftButton: boolean;
  readonly middleButton: boolean;
  readonly rightButton: boolean;
  readonly buttons: number;
  readonly clientX: number;
  readonly clientY: number;
  readonly pageX: number;
  readonly pageY: number;
  readonly ctrlKey: boolean;
  readonly shiftKey: boolean;
  readonly altKey: boolean;
  readonly metaKey: boolean;

  constructor(readonly browserEvent: MouseEvent) {
    const targetWindow = getWindow(browserEvent);
    this.target = browserEvent.target;
    this.leftButton = browserEvent.button === 0;
    this.middleButton = browserEvent.button === 1;
    this.rightButton = browserEvent.button === 2;
    this.buttons = browserEvent.buttons;
    this.clientX = browserEvent.clientX;
    this.clientY = browserEvent.clientY;
    this.pageX = browserEvent.clientX + targetWindow.scrollX;
    this.pageY = browserEvent.clientY + targetWindow.scrollY;
    this.ctrlKey = browserEvent.ctrlKey;
    this.shiftKey = browserEvent.shiftKey;
    this.altKey = browserEvent.altKey;
    this.metaKey = browserEvent.metaKey;
  }

  stop(options?: {
    readonly preventDefault?: boolean;
    readonly immediate?: boolean;
  }): void {
    stopEvent(this.browserEvent, options);
  }
}

export class StandardPointerEvent extends StandardMouseEvent {
  readonly pointerId: number;
  readonly pointerType: string;
  readonly pressure: number;

  constructor(override readonly browserEvent: PointerEvent) {
    super(browserEvent);
    this.pointerId = browserEvent.pointerId;
    this.pointerType = browserEvent.pointerType;
    this.pressure = browserEvent.pressure;
  }
}
