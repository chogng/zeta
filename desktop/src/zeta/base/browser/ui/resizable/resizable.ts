import { Dimension, type IDimension } from "../../geometry.js";
import { observeElementSize } from "../../observer.js";
import { type Event, Emitter } from "../../../common/event.js";
import { DisposableOwner, type IDisposable } from "../../../common/lifecycle.js";
import { Sash } from "../sash/sash.js";

/** A control whose geometry is driven by an external container layout. */
export interface IResizable {
  /** Applies the current container dimension to the control. */
  layout(dimension: IDimension): void;
}

/** Connects a container dimension event to a generic resizable control. */
export function bindResizableLayout(event: Event<IDimension>, resizable: IResizable): IDisposable {
  return event((dimension) => resizable.layout(dimension));
}

export interface IResizeEvent {
  readonly dimension: Dimension;
  readonly done: boolean;
  readonly north?: boolean;
  readonly east?: boolean;
  readonly south?: boolean;
  readonly west?: boolean;
}

/** A four-edge resize surface for floating or otherwise independently sized UI. */
export class ResizableHTMLElement extends DisposableOwner {
  readonly domNode: HTMLDivElement;

  private readonly _onDidWillResize = this.own(new Emitter<void>());
  readonly onDidWillResize: Event<void> = this._onDidWillResize.event;
  private readonly _onDidResize = this.own(new Emitter<IResizeEvent>());
  readonly onDidResize: Event<IResizeEvent> = this._onDidResize.event;

  private readonly northSash: Sash;
  private readonly eastSash: Sash;
  private readonly southSash: Sash;
  private readonly westSash: Sash;

  private sizeValue = Dimension.Zero;
  private minSizeValue = Dimension.Zero;
  private maxSizeValue = new Dimension(Number.MAX_SAFE_INTEGER, Number.MAX_SAFE_INTEGER);
  private preferredSizeValue: Dimension | undefined;
  private resizeStart: Dimension | undefined;
  private deltaX = 0;
  private deltaY = 0;

  constructor(ownerDocument: Document = document) {
    super();
    this.domNode = ownerDocument.createElement("div");
    this.domNode.className = "zeta-resizable";
    this.defer(() => this.domNode.remove());

    this.northSash = this.own(new Sash("horizontal", ownerDocument));
    this.eastSash = this.own(new Sash("vertical", ownerDocument));
    this.southSash = this.own(new Sash("horizontal", ownerDocument));
    this.westSash = this.own(new Sash("vertical", ownerDocument));
    this.domNode.append(
      this.northSash.element,
      this.eastSash.element,
      this.southSash.element,
      this.westSash.element,
    );

    this.connectSash(this.northSash, "north");
    this.connectSash(this.eastSash, "east");
    this.connectSash(this.southSash, "south");
    this.connectSash(this.westSash, "west");
    this.enableSashes(true, true, true, true);
    this.layoutSashes();
  }

  enableSashes(north: boolean, east: boolean, south: boolean, west: boolean): void {
    this.setSashEnabled(this.northSash, north);
    this.setSashEnabled(this.eastSash, east);
    this.setSashEnabled(this.southSash, south);
    this.setSashEnabled(this.westSash, west);
  }

  layout(height: number = this.size.height, width: number = this.size.width): void {
    assertNonNegativeFinite(height, "height");
    assertNonNegativeFinite(width, "width");
    const nextHeight = clamp(height, this.minSize.height, this.maxSize.height);
    const nextWidth = clamp(width, this.minSize.width, this.maxSize.width);
    const nextSize = new Dimension(nextWidth, nextHeight);
    if (Dimension.equals(nextSize, this.sizeValue)) return;

    this.domNode.style.height = `${nextHeight}px`;
    this.domNode.style.width = `${nextWidth}px`;
    this.sizeValue = nextSize;
    this.layoutSashes();
  }

  clearSashHoverState(): void {
    this.northSash.clearSashHoverState();
    this.eastSash.clearSashHoverState();
    this.southSash.clearSashHoverState();
    this.westSash.clearSashHoverState();
  }

  get size(): Dimension {
    return this.sizeValue;
  }

  set maxSize(value: Dimension) {
    assertSize(value, "maximum size", true);
    if (value.width < this.minSize.width || value.height < this.minSize.height) {
      throw new RangeError("Resizable maximum size must not be smaller than its minimum size");
    }
    this.maxSizeValue = value;
  }

  get maxSize(): Dimension {
    return this.maxSizeValue;
  }

  set minSize(value: Dimension) {
    assertSize(value, "minimum size");
    if (value.width > this.maxSize.width || value.height > this.maxSize.height) {
      throw new RangeError("Resizable minimum size must not exceed its maximum size");
    }
    this.minSizeValue = value;
  }

  get minSize(): Dimension {
    return this.minSizeValue;
  }

  set preferredSize(value: Dimension | undefined) {
    if (value) assertSize(value, "preferred size");
    this.preferredSizeValue = value;
  }

  get preferredSize(): Dimension | undefined {
    return this.preferredSizeValue;
  }

  private connectSash(sash: Sash, edge: "north" | "east" | "south" | "west"): void {
    this.own(sash.onDidStart(() => {
      if (this.resizeStart !== undefined) return;
      this._onDidWillResize.fire();
      this.resizeStart = this.sizeValue;
      this.deltaX = 0;
      this.deltaY = 0;
    }));
    this.own(sash.onDidChange((event) => {
      if (this.resizeStart === undefined) return;
      if (edge === "east") this.deltaX = event.delta;
      if (edge === "west") this.deltaX = -event.delta;
      if (edge === "south") this.deltaY = event.delta;
      if (edge === "north") this.deltaY = -event.delta;
      this.layout(
        this.resizeStart.height + this.deltaY,
        this.resizeStart.width + this.deltaX,
      );
      this._onDidResize.fire({
        dimension: this.sizeValue,
        done: false,
        [edge]: true,
      });
    }));
    this.own(sash.onDidReset(() => {
      if (this.preferredSize === undefined) return;
      const height = edge === "north" || edge === "south"
        ? this.preferredSize.height
        : this.size.height;
      const width = edge === "east" || edge === "west"
        ? this.preferredSize.width
        : this.size.width;
      this.layout(height, width);
      this._onDidResize.fire({ dimension: this.sizeValue, done: true });
    }));
    this.own(sash.onDidEnd(() => {
      if (this.resizeStart === undefined) return;
      this.resizeStart = undefined;
      this.deltaX = 0;
      this.deltaY = 0;
      this._onDidResize.fire({ dimension: this.sizeValue, done: true });
    }));
  }

  private setSashEnabled(sash: Sash, enabled: boolean): void {
    sash.element.hidden = !enabled;
    sash.element.tabIndex = enabled ? 0 : -1;
    sash.element.setAttribute("aria-disabled", String(!enabled));
  }

  private layoutSashes(): void {
    setSashBounds(this.northSash, 0, 0, this.size.width, 1);
    setSashBounds(this.eastSash, this.size.width, 0, 1, this.size.height);
    setSashBounds(this.southSash, 0, this.size.height, this.size.width, 1);
    setSashBounds(this.westSash, 0, 0, 1, this.size.height);
  }
}

/** Compatibility wrapper for callers that use the browser-native resize surface. */
export class Resizable extends DisposableOwner {
  readonly element: HTMLDivElement;

  constructor(onResize?: (size: IDimension) => void, ownerDocument: Document = document) {
    super();
    this.element = ownerDocument.createElement("div");
    this.element.className = "zeta-resizable";
    this.element.style.resize = "both";
    this.element.style.overflow = "auto";
    this.defer(() => this.element.remove());
    this.own(observeElementSize(this.element, (size) => onResize?.(size)));
  }
}

function setSashBounds(sash: Sash, left: number, top: number, width: number, height: number): void {
  sash.element.style.left = `${left}px`;
  sash.element.style.top = `${top}px`;
  sash.element.style.width = `${width}px`;
  sash.element.style.height = `${height}px`;
}

function clamp(value: number, minimum: number, maximum: number): number {
  return Math.max(minimum, Math.min(maximum, value));
}

function assertSize(value: IDimension, name: string, allowInfinity = false): void {
  const validWidth = value.width >= 0 && (allowInfinity || Number.isFinite(value.width));
  const validHeight = value.height >= 0 && (allowInfinity || Number.isFinite(value.height));
  if (!validWidth || !validHeight) {
    throw new RangeError(`Resizable ${name} must be non-negative${allowInfinity ? "" : " and finite"}`);
  }
}

function assertNonNegativeFinite(value: number, name: string): void {
  if (!Number.isFinite(value) || value < 0) {
    throw new RangeError(`Resizable ${name} must be non-negative and finite`);
  }
}
