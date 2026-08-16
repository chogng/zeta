import { Emitter, type Event } from "../../../common/event.js";
import { type IDisposable, DisposableOwner, ResettableDisposableGroup } from "../../../common/lifecycle.js";
import { Sash, type SashDragEvent, type SashPresentation } from "../sash/sash.js";
import { findFirstSnapIndex, getSashState, solveSashResize, type SplitViewResizeItem } from "./splitviewResize.js";

export type SplitViewOrientation = "horizontal" | "vertical";
export type SplitViewLayoutPriority = "low" | "normal" | "high";

export interface SplitViewOptions {
  /** Optional Sash presentation shared by separators created for this view. */
  readonly sashPresentation?: SashPresentation;
  /** Whether a snap view at the leading outer edge can be restored by its sash. */
  readonly startSnappingEnabled?: boolean;
  /** Whether a snap view at the trailing outer edge can be restored by its sash. */
  readonly endSnappingEnabled?: boolean;
}

/**
 * A view hosted by SplitView.
 *
 * Constraints and layout use the SplitView's primary axis. Implementations
 * receive their position on that axis and the available orthogonal size.
 */
export interface ISplitViewView {
  readonly element: HTMLElement;
  readonly minimumSize: number;
  readonly maximumSize: number;
  readonly priority?: SplitViewLayoutPriority;
  /** Whether this pane may render interaction surfaces beyond its own bounds. */
  readonly paneOverflow?: "hidden" | "visible";
  /** Whether dragging through the minimum size may collapse this view. */
  readonly snap?: boolean;
  readonly onDidChange?: Event<number | undefined>;
  layout(size: number, offset: number, orthogonalSize: number): void;
  setVisible?(visible: boolean): void;
}

/** Controls the initial placement of a newly added SplitView view. */
export type SplitViewSizing =
  | number
  | { readonly type: "distribute" }
  | { readonly type: "split"; readonly index: number }
  | { readonly type: "invisible"; readonly cachedVisibleSize: number };

interface ViewItem {
  readonly view: ISplitViewView;
  readonly container: HTMLDivElement;
  size: number;
  visible: boolean;
  cachedVisibleSize: number | undefined;
  changeListener: IDisposable | undefined;
}

interface SashItem {
  readonly sash: Sash;
  readonly boundaryIndex: number;
}

interface SashDragState {
  snapshot: readonly SplitViewResizeItem[];
  baseline: number;
  altKey: boolean | undefined;
}

/** A constrained, explicit-pixel layout with accessible resize sashes. */
export class SplitView extends DisposableOwner {
  readonly element: HTMLDivElement;
  private readonly items: ViewItem[] = [];
  private readonly sashes = this.own(new ResettableDisposableGroup());
  private readonly sashItems: SashItem[] = [];
  private readonly _onDidChangeViewSizes = this.own(new Emitter<void>());
  private readonly _onDidSashReset = this.own(new Emitter<number>());
  private size = 0;
  private orthogonalSize = 0;
  private didLayout = false;
  private _startSnappingEnabled: boolean;
  private _endSnappingEnabled: boolean;
  private _orthogonalStartSash: Sash | undefined;
  private _orthogonalEndSash: Sash | undefined;

  readonly onDidChangeViewSizes: Event<void> =
    this._onDidChangeViewSizes.event;
  readonly onDidSashReset: Event<number> = this._onDidSashReset.event;

  constructor(
    readonly orientation: SplitViewOrientation,
    ownerDocument: Document = document,
    private readonly options: SplitViewOptions = {},
  ) {
    super();
    const element = ownerDocument.createElement("div");
    this.element = element;
    this.defer(() => element.remove());
    element.className = `zeta-split-view zeta-split-view-${orientation}`;
    this._startSnappingEnabled = options.startSnappingEnabled ?? true;
    this._endSnappingEnabled = options.endSnappingEnabled ?? true;
  }

  get viewCount(): number {
    return this.items.length;
  }

  get startSnappingEnabled(): boolean {
    return this._startSnappingEnabled;
  }

  set startSnappingEnabled(enabled: boolean) {
    if (this._startSnappingEnabled === enabled) return;
    this._startSnappingEnabled = enabled;
    this.positionSashes();
  }

  get endSnappingEnabled(): boolean {
    return this._endSnappingEnabled;
  }

  set endSnappingEnabled(enabled: boolean) {
    if (this._endSnappingEnabled === enabled) return;
    this._endSnappingEnabled = enabled;
    this.positionSashes();
  }

  get orthogonalStartSash(): Sash | undefined {
    return this._orthogonalStartSash;
  }

  set orthogonalStartSash(sash: Sash | undefined) {
    this._orthogonalStartSash = sash;
    for (const item of this.sashItems) item.sash.orthogonalStartSash = sash;
  }

  get orthogonalEndSash(): Sash | undefined {
    return this._orthogonalEndSash;
  }

  set orthogonalEndSash(sash: Sash | undefined) {
    this._orthogonalEndSash = sash;
    for (const item of this.sashItems) item.sash.orthogonalEndSash = sash;
  }

  getSash(boundaryIndex: number): Sash | undefined {
    return this.sashItems.find((item) => item.boundaryIndex === boundaryIndex)?.sash;
  }

  get minimumSize(): number {
    return this.visibleItems().reduce(
      (total, item) => total + item.view.minimumSize,
      0,
    );
  }

  get maximumSize(): number {
    return this.visibleItems().reduce(
      (total, item) => total + item.view.maximumSize,
      0,
    );
  }

  addView(
    view: ISplitViewView,
    sizing: SplitViewSizing = { type: "distribute" },
    index = this.items.length,
  ): void {
    if (this.items.some((item) => item.view === view)) {
      throw new Error("SplitView cannot contain the same view twice");
    }
    if (!Number.isInteger(index) || index < 0 || index > this.items.length) {
      throw new RangeError(`SplitView view index is out of range: ${index}`);
    }
    validateViewConstraints(view);
    const resolved = this.resolveSizing(sizing);
    const container = this.element.ownerDocument.createElement("div");
    container.className = "zeta-split-view-pane";
    container.classList.toggle("zeta-split-view-pane-overflow-visible", view.paneOverflow === "visible");
    container.append(view.element);
    const item: ViewItem = {
      view,
      container,
      size: resolved.visible ? clamp(
        resolved.size,
        view.minimumSize,
        view.maximumSize,
      ) : 0,
      visible: resolved.visible,
      cachedVisibleSize: resolved.visible ? undefined : resolved.size,
      changeListener: undefined,
    };
    this.items.splice(index, 0, item);
    const next = this.element.children[index];
    this.element.insertBefore(container, next ?? null);
    container.hidden = !item.visible;
    view.setVisible?.(item.visible);
    if (view.onDidChange) {
      item.changeListener = this.own(view.onDidChange((preferredSize) => {
        validateViewConstraints(view);
        if (!this.didLayout && preferredSize === undefined) return;
        if (preferredSize === undefined) {
          this.fitToSize();
        } else {
          this.resizeView(this.indexOf(view), preferredSize);
          return;
        }
        this.render();
      }));
    }
    if (this.didLayout) this.fitToSize();
    this.rebuildSashes();
    if (this.didLayout) this.render();
  }

  removeView(index: number): ISplitViewView {
    const item = this.item(index);
    this.items.splice(index, 1);
    item.changeListener?.dispose();
    item.container.remove();
    if (this.didLayout) this.fitToSize();
    this.rebuildSashes();
    if (this.didLayout) this.render();
    this._onDidChangeViewSizes.fire();
    return item.view;
  }

  layout(size: number, orthogonalSize: number): void {
    assertNonNegativeFinite(size, "size");
    assertNonNegativeFinite(orthogonalSize, "orthogonal size");
    this.size = size;
    this.orthogonalSize = orthogonalSize;
    this.didLayout = true;
    this.fitToSize();
    if (this.didLayout) this.render();
  }

  getViewSize(index: number): number {
    return this.item(index).size;
  }

  getViewCachedVisibleSize(index: number): number | undefined {
    return this.item(index).cachedVisibleSize;
  }

  isViewVisible(index: number): boolean {
    return this.item(index).visible;
  }

  setViewVisible(index: number, visible: boolean): void {
    const item = this.item(index);
    if (item.visible === visible) return;
    this.setItemVisible(item, visible);
    if (this.didLayout) this.render();
    this._onDidChangeViewSizes.fire();
  }

  resizeView(index: number, requestedSize: number): void {
    assertNonNegativeFinite(requestedSize, "view size");
    const item = this.item(index);
    if (!item.visible) {
      item.cachedVisibleSize = clamp(
        requestedSize,
        item.view.minimumSize,
        item.view.maximumSize,
      );
      return;
    }
    item.size = clamp(
      requestedSize,
      item.view.minimumSize,
      item.view.maximumSize,
    );
    if (this.didLayout) this.fitToSize(new Set([item]));
    if (this.didLayout) this.render();
    this._onDidChangeViewSizes.fire();
  }

  distributeViewSizes(): void {
    if (!this.didLayout) return;
    const flexible = this.visibleItems().filter(isResizable);
    if (flexible.length === 0) return;
    const fixedSize = this.visibleItems()
      .filter((item) => !isResizable(item))
      .reduce((total, item) => total + item.size, 0);
    const target = Math.max(0, this.size - fixedSize);
    const share = target / flexible.length;
    for (const item of flexible) {
      item.size = clamp(
        share,
        item.view.minimumSize,
        item.view.maximumSize,
      );
    }
    this.fitToSize();
    this.render();
    this._onDidChangeViewSizes.fire();
  }

  private resolveSizing(
    sizing: SplitViewSizing,
  ): { readonly size: number; readonly visible: boolean } {
    if (typeof sizing === "number") {
      assertNonNegativeFinite(sizing, "initial view size");
      return { size: sizing, visible: true };
    }
    if (sizing.type === "invisible") {
      assertNonNegativeFinite(
        sizing.cachedVisibleSize,
        "cached visible size",
      );
      return { size: sizing.cachedVisibleSize, visible: false };
    }
    if (sizing.type === "split") {
      const target = this.item(sizing.index);
      return { size: target.size / 2, visible: true };
    }
    if (sizing.type !== "distribute") {
      throw new TypeError("SplitView sizing has an unknown type");
    }
    const visible = this.visibleItems();
    return {
      size: visible.length === 0
        ? this.size
        : visible.reduce((total, item) => total + item.size, 0) /
          visible.length,
      visible: true,
    };
  }

  private fitToSize(protectedItems: ReadonlySet<ViewItem> = new Set()): void {
    const visible = this.visibleItems();
    for (const item of visible) {
      validateViewConstraints(item.view);
      item.size = clamp(
        item.size,
        item.view.minimumSize,
        item.view.maximumSize,
      );
    }
    let delta = this.size -
      visible.reduce((total, item) => total + item.size, 0);
    delta = distributeByPriority(
      visible.filter((item) => !protectedItems.has(item)),
      delta,
    );
    if (Math.abs(delta) > 0.001) {
      distributeByPriority(
        visible.filter((item) => protectedItems.has(item)),
        delta,
      );
    }
  }

  private render(): void {
    let offset = 0;
    for (const item of this.items) {
      if (!item.visible) continue;
      const primarySize = Math.max(0, item.size);
      if (this.orientation === "horizontal") {
        item.container.style.left = `${offset}px`;
        item.container.style.top = "0px";
        item.container.style.width = `${primarySize}px`;
        item.container.style.height = `${this.orthogonalSize}px`;
      } else {
        item.container.style.left = "0px";
        item.container.style.top = `${offset}px`;
        item.container.style.width = `${this.orthogonalSize}px`;
        item.container.style.height = `${primarySize}px`;
      }
      item.view.layout(primarySize, offset, this.orthogonalSize);
      offset += primarySize;
    }
    this.positionSashes();
  }

  private rebuildSashes(): void {
    this.sashes.clear();
    this.sashItems.length = 0;
    for (let boundaryIndex = 0; boundaryIndex < this.items.length - 1; boundaryIndex += 1) {
      if (this.canResizeAtBoundary(boundaryIndex)) {
        this.addSash(boundaryIndex);
      }
    }
    this.positionSashes();
  }

  private addSash(boundaryIndex: number): void {
    const sash = this.sashes.add(new Sash(
      this.orientation === "horizontal" ? "vertical" : "horizontal",
      this.element.ownerDocument,
      this.options.sashPresentation,
    ));
    sash.orthogonalStartSash = this._orthogonalStartSash;
    sash.orthogonalEndSash = this._orthogonalEndSash;
    this.sashItems.push({ sash, boundaryIndex });
    let dragState: SashDragState | undefined;
    this.sashes.add(sash.onDidStart(() => {
      dragState = { snapshot: this.getResizeItems(), baseline: 0, altKey: undefined };
    }));
    this.sashes.add(sash.onDidChange((event) => {
      if (!dragState) return;
      if (event.input === "pointer" && dragState.altKey !== undefined && dragState.altKey !== event.altKey) {
        dragState.snapshot = this.getResizeItems();
        dragState.baseline = event.delta;
      }
      if (event.input === "pointer") dragState.altKey = event.altKey;
      this.resizeAtBoundary(boundaryIndex, dragState.snapshot, {
        ...event,
        delta: event.input === "pointer" ? event.delta - dragState.baseline : event.delta,
      });
    }));
    this.sashes.add(sash.onDidEnd(() => {
      dragState = undefined;
    }));
    this.sashes.add(sash.onDidReset(() => this.resetSash(boundaryIndex)));
    this.element.append(sash.element);
  }

  private resizeAtBoundary(boundaryIndex: number, snapshot: readonly SplitViewResizeItem[], event: SashDragEvent): void {
    const resizedItems = solveSashResize(snapshot, {
      boundaryIndex,
      delta: event.delta,
      input: event.input,
      altKey: event.altKey,
      startSnappingEnabled: this.startSnappingEnabled,
      endSnappingEnabled: this.endSnappingEnabled,
    });
    const visibilityChanges: Array<{ readonly item: ViewItem; readonly visible: boolean }> = [];
    for (const [index, resized] of resizedItems.entries()) {
      const item = this.item(index);
      if (item.visible !== resized.visible) {
        item.visible = resized.visible;
        item.container.hidden = !resized.visible;
        visibilityChanges.push({ item, visible: resized.visible });
      }
      item.size = resized.size;
      item.cachedVisibleSize = resized.cachedVisibleSize;
    }
    for (const { item, visible } of visibilityChanges) {
      item.view.setVisible?.(visible);
    }
    this.fitToSize();
    this.render();
    this._onDidChangeViewSizes.fire();
  }

  private resetSash(boundaryIndex: number): void {
    const items = this.getResizeItems();
    const before = findFirstSnapIndex(items, Array.from({ length: boundaryIndex + 1 }, (_, index) => boundaryIndex - index));
    const after = findFirstSnapIndex(items, Array.from({ length: items.length - boundaryIndex - 1 }, (_, index) => boundaryIndex + index + 1));
    if (before !== undefined && !items[before]!.visible) return;
    if (after !== undefined && !items[after]!.visible) return;
    this._onDidSashReset.fire(boundaryIndex);
  }

  private setItemVisible(
    item: ViewItem,
    visible: boolean,
    visibleSize?: number,
    fit = true,
  ): void {
    if (visible) {
      item.visible = true;
      item.size = clamp(
        visibleSize ?? item.cachedVisibleSize ?? item.view.minimumSize,
        item.view.minimumSize,
        item.view.maximumSize,
      );
      item.cachedVisibleSize = undefined;
      item.container.hidden = false;
      item.view.setVisible?.(true);
      if (fit && this.didLayout) this.fitToSize(new Set([item]));
      return;
    }
    item.cachedVisibleSize = visibleSize ?? item.size;
    item.size = 0;
    item.visible = false;
    item.container.hidden = true;
    item.view.setVisible?.(false);
    if (fit && this.didLayout) this.fitToSize();
  }

  private getResizeItems(): SplitViewResizeItem[] {
    return this.items.map((item) => ({
      size: item.size,
      cachedVisibleSize: item.cachedVisibleSize,
      minimumSize: item.view.minimumSize,
      maximumSize: item.view.maximumSize,
      visible: item.visible,
      snap: item.view.snap ?? false,
    }));
  }

  private canResizeAtBoundary(boundaryIndex: number): boolean {
    const canResize = (item: ViewItem) => isResizable(item) || item.view.snap === true;
    return this.items.slice(0, boundaryIndex + 1).some(canResize) && this.items.slice(boundaryIndex + 1).some(canResize);
  }

  private positionSashes(): void {
    const resizeItems = this.getResizeItems();
    for (const { sash, boundaryIndex: previousIndex } of this.sashItems) {
      let position = 0;
      for (let index = 0; index <= previousIndex; index += 1) {
        const item = this.items[index];
        if (item?.visible) position += item.size;
      }
      if (this.orientation === "horizontal") {
        sash.element.style.left = `${position}px`;
        sash.element.style.top = "0px";
        sash.element.style.height = `${this.orthogonalSize}px`;
      } else {
        sash.element.style.left = "0px";
        sash.element.style.top = `${position}px`;
        sash.element.style.width = `${this.orthogonalSize}px`;
      }
      sash.state = getSashState(resizeItems, previousIndex, this.startSnappingEnabled, this.endSnappingEnabled);
      const previous = this.items[previousIndex];
      const next = this.items[previousIndex + 1];
      if (previous && next && previous.visible && next.visible) {
        sash.element.setAttribute(
          "aria-valuemin",
          String(previous.view.minimumSize),
        );
        sash.element.setAttribute(
          "aria-valuemax",
          String(Math.min(
            previous.view.maximumSize,
            previous.size + next.size - next.view.minimumSize,
          )),
        );
        sash.element.setAttribute("aria-valuenow", String(previous.size));
      } else {
        sash.element.removeAttribute("aria-valuemin");
        sash.element.removeAttribute("aria-valuemax");
        sash.element.removeAttribute("aria-valuenow");
      }
    }
  }

  private visibleItems(): ViewItem[] {
    return this.items.filter((item) => item.visible);
  }

  private indexOf(view: ISplitViewView): number {
    const index = this.items.findIndex((item) => item.view === view);
    if (index < 0) throw new Error("SplitView view is not registered");
    return index;
  }

  private item(index: number): ViewItem {
    const item = this.items[index];
    if (!item) throw new RangeError(`SplitView view index is out of range: ${index}`);
    return item;
  }
}

function distributeDelta(items: readonly ViewItem[], delta: number): number {
  let candidates = items.filter((item) =>
    delta > 0
      ? item.size < item.view.maximumSize
      : item.size > item.view.minimumSize
  );
  while (candidates.length > 0 && Math.abs(delta) > 0.001) {
    const share = delta / candidates.length;
    let applied = 0;
    for (const item of candidates) {
      const next = clamp(
        item.size + share,
        item.view.minimumSize,
        item.view.maximumSize,
      );
      applied += next - item.size;
      item.size = next;
    }
    if (Math.abs(applied) < 0.001) break;
    delta -= applied;
    candidates = candidates.filter((item) =>
      delta > 0
        ? item.size < item.view.maximumSize
        : item.size > item.view.minimumSize
    );
  }
  return delta;
}

function distributeByPriority(
  items: readonly ViewItem[],
  delta: number,
): number {
  for (const priority of ["high", "normal", "low"] as const) {
    delta = distributeDelta(
      items.filter((item) => (item.view.priority ?? "normal") === priority),
      delta,
    );
    if (Math.abs(delta) <= 0.001) break;
  }
  return delta;
}

function validateViewConstraints(view: ISplitViewView): void {
  assertNonNegativeFinite(view.minimumSize, "view minimum size");
  if (
    typeof view.maximumSize !== "number" ||
    Number.isNaN(view.maximumSize) ||
    view.maximumSize < view.minimumSize
  ) {
    throw new RangeError(
      "SplitView view maximum size must be at least its minimum size",
    );
  }
}

function isResizable(item: ViewItem): boolean {
  return item.view.minimumSize < item.view.maximumSize;
}

function assertNonNegativeFinite(value: number, name: string): void {
  if (!Number.isFinite(value) || value < 0) {
    throw new RangeError(
      `SplitView ${name} must be a non-negative finite number`,
    );
  }
}

function clamp(value: number, minimum: number, maximum: number): number {
  return Math.min(Math.max(value, minimum), maximum);
}
