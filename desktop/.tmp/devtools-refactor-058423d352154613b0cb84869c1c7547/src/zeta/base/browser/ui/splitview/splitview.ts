import { Emitter, type Event } from "../../../common/event.js";
import {
  type IDisposable,
  DisposableOwner,
  ResettableDisposableGroup,
} from "../../../common/lifecycle.js";
import { Sash } from "../sash/sash.js";

export type SplitViewOrientation = "horizontal" | "vertical";
export type SplitViewLayoutPriority = "low" | "normal" | "high";

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

interface DragSnapshot {
  readonly previousSize: number;
  readonly nextSize: number;
}

/** A constrained, explicit-pixel layout with accessible resize sashes. */
export class SplitView extends DisposableOwner {
  readonly element: HTMLDivElement;
  readonly #items: ViewItem[] = [];
  readonly #sashes = this.own(new ResettableDisposableGroup());
  readonly #onDidChangeViewSizes = this.own(new Emitter<void>());
  #size = 0;
  #orthogonalSize = 0;
  #didLayout = false;

  readonly onDidChangeViewSizes: Event<void> =
    this.#onDidChangeViewSizes.event;

  constructor(
    readonly orientation: SplitViewOrientation,
    ownerDocument: Document = document,
  ) {
    super();
    const element = ownerDocument.createElement("div");
    this.element = element;
    this.defer(() => element.remove());
    element.className = `zeta-split-view zeta-split-view-${orientation}`;
  }

  get viewCount(): number {
    return this.#items.length;
  }

  get minimumSize(): number {
    return this.#visibleItems().reduce(
      (total, item) => total + item.view.minimumSize,
      0,
    );
  }

  get maximumSize(): number {
    return this.#visibleItems().reduce(
      (total, item) => total + item.view.maximumSize,
      0,
    );
  }

  addView(
    view: ISplitViewView,
    sizing: SplitViewSizing = { type: "distribute" },
    index = this.#items.length,
  ): void {
    if (this.#items.some((item) => item.view === view)) {
      throw new Error("SplitView cannot contain the same view twice");
    }
    if (!Number.isInteger(index) || index < 0 || index > this.#items.length) {
      throw new RangeError(`SplitView view index is out of range: ${index}`);
    }
    validateViewConstraints(view);
    const resolved = this.#resolveSizing(sizing);
    const container = this.element.ownerDocument.createElement("div");
    container.className = "zeta-split-view-pane";
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
    this.#items.splice(index, 0, item);
    const next = this.element.children[index];
    this.element.insertBefore(container, next ?? null);
    container.hidden = !item.visible;
    view.setVisible?.(item.visible);
    if (view.onDidChange) {
      item.changeListener = this.own(view.onDidChange((preferredSize) => {
        validateViewConstraints(view);
        if (!this.#didLayout && preferredSize === undefined) return;
        if (preferredSize === undefined) {
          this.#fitToSize();
        } else {
          this.resizeView(this.#indexOf(view), preferredSize);
          return;
        }
        this.#render();
      }));
    }
    if (this.#didLayout) this.#fitToSize();
    this.#rebuildSashes();
    if (this.#didLayout) this.#render();
  }

  removeView(index: number): ISplitViewView {
    const item = this.#item(index);
    this.#items.splice(index, 1);
    item.changeListener?.dispose();
    item.container.remove();
    if (this.#didLayout) this.#fitToSize();
    this.#rebuildSashes();
    if (this.#didLayout) this.#render();
    this.#onDidChangeViewSizes.fire();
    return item.view;
  }

  layout(size: number, orthogonalSize: number): void {
    assertNonNegativeFinite(size, "size");
    assertNonNegativeFinite(orthogonalSize, "orthogonal size");
    this.#size = size;
    this.#orthogonalSize = orthogonalSize;
    this.#didLayout = true;
    this.#fitToSize();
    if (this.#didLayout) this.#render();
  }

  getViewSize(index: number): number {
    return this.#item(index).size;
  }

  getViewCachedVisibleSize(index: number): number | undefined {
    return this.#item(index).cachedVisibleSize;
  }

  isViewVisible(index: number): boolean {
    return this.#item(index).visible;
  }

  setViewVisible(index: number, visible: boolean): void {
    const item = this.#item(index);
    if (item.visible === visible) return;
    if (visible) {
      item.visible = true;
      item.size = clamp(
        item.cachedVisibleSize ?? item.view.minimumSize,
        item.view.minimumSize,
        item.view.maximumSize,
      );
      item.cachedVisibleSize = undefined;
      item.container.hidden = false;
      item.view.setVisible?.(true);
      if (this.#didLayout) this.#fitToSize(new Set([item]));
    } else {
      item.cachedVisibleSize = item.size;
      item.size = 0;
      item.visible = false;
      item.container.hidden = true;
      item.view.setVisible?.(false);
      if (this.#didLayout) this.#fitToSize();
    }
    this.#rebuildSashes();
    if (this.#didLayout) this.#render();
    this.#onDidChangeViewSizes.fire();
  }

  resizeView(index: number, requestedSize: number): void {
    assertNonNegativeFinite(requestedSize, "view size");
    const item = this.#item(index);
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
    if (this.#didLayout) this.#fitToSize(new Set([item]));
    if (this.#didLayout) this.#render();
    this.#onDidChangeViewSizes.fire();
  }

  distributeViewSizes(): void {
    if (!this.#didLayout) return;
    const flexible = this.#visibleItems().filter(isResizable);
    if (flexible.length === 0) return;
    const fixedSize = this.#visibleItems()
      .filter((item) => !isResizable(item))
      .reduce((total, item) => total + item.size, 0);
    const target = Math.max(0, this.#size - fixedSize);
    const share = target / flexible.length;
    for (const item of flexible) {
      item.size = clamp(
        share,
        item.view.minimumSize,
        item.view.maximumSize,
      );
    }
    this.#fitToSize();
    this.#render();
    this.#onDidChangeViewSizes.fire();
  }

  #resolveSizing(
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
      const target = this.#item(sizing.index);
      return { size: target.size / 2, visible: true };
    }
    if (sizing.type !== "distribute") {
      throw new TypeError("SplitView sizing has an unknown type");
    }
    const visible = this.#visibleItems();
    return {
      size: visible.length === 0
        ? this.#size
        : visible.reduce((total, item) => total + item.size, 0) /
          visible.length,
      visible: true,
    };
  }

  #fitToSize(protectedItems: ReadonlySet<ViewItem> = new Set()): void {
    const visible = this.#visibleItems();
    for (const item of visible) {
      validateViewConstraints(item.view);
      item.size = clamp(
        item.size,
        item.view.minimumSize,
        item.view.maximumSize,
      );
    }
    let delta = this.#size -
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

  #render(): void {
    let offset = 0;
    for (const item of this.#items) {
      if (!item.visible) continue;
      const primarySize = Math.max(0, item.size);
      if (this.orientation === "horizontal") {
        item.container.style.left = `${offset}px`;
        item.container.style.top = "0px";
        item.container.style.width = `${primarySize}px`;
        item.container.style.height = `${this.#orthogonalSize}px`;
      } else {
        item.container.style.left = "0px";
        item.container.style.top = `${offset}px`;
        item.container.style.width = `${this.#orthogonalSize}px`;
        item.container.style.height = `${primarySize}px`;
      }
      item.view.layout(primarySize, offset, this.#orthogonalSize);
      offset += primarySize;
    }
    this.#positionSashes();
  }

  #rebuildSashes(): void {
    this.#sashes.clear();
    for (const sash of this.element.querySelectorAll(":scope > .zeta-sash")) {
      sash.remove();
    }
    const visible = this.#items
      .map((item, index) => ({ item, index }))
      .filter(({ item }) => item.visible);
    for (let index = 1; index < visible.length; index += 1) {
      const previous = visible[index - 1]!;
      const next = visible[index]!;
      if (!isResizable(previous.item) || !isResizable(next.item)) continue;
      this.#addSash(previous.index, next.index);
    }
    this.#positionSashes();
  }

  #addSash(previousIndex: number, nextIndex: number): void {
    const sash = this.#sashes.add(new Sash(
      this.orientation === "horizontal" ? "vertical" : "horizontal",
      this.element.ownerDocument,
    ));
    sash.element.dataset.previousViewIndex = String(previousIndex);
    let snapshot: DragSnapshot | undefined;
    this.#sashes.add(sash.onDidStart(() => {
      snapshot = {
        previousSize: this.#item(previousIndex).size,
        nextSize: this.#item(nextIndex).size,
      };
    }));
    this.#sashes.add(sash.onDidChange(({ delta }) => {
      if (!snapshot) return;
      this.#resizeAdjacent(
        previousIndex,
        nextIndex,
        snapshot,
        delta,
      );
    }));
    this.#sashes.add(sash.onDidEnd(() => {
      snapshot = undefined;
    }));
    this.element.append(sash.element);
  }

  #resizeAdjacent(
    previousIndex: number,
    nextIndex: number,
    snapshot: DragSnapshot,
    delta: number,
  ): void {
    const previous = this.#item(previousIndex);
    const next = this.#item(nextIndex);
    const minimumDelta = Math.max(
      previous.view.minimumSize - snapshot.previousSize,
      snapshot.nextSize - next.view.maximumSize,
    );
    const maximumDelta = Math.min(
      previous.view.maximumSize - snapshot.previousSize,
      snapshot.nextSize - next.view.minimumSize,
    );
    if (minimumDelta > maximumDelta) return;
    const constrained = clamp(delta, minimumDelta, maximumDelta);
    previous.size = snapshot.previousSize + constrained;
    next.size = snapshot.nextSize - constrained;
    this.#render();
    this.#onDidChangeViewSizes.fire();
  }

  #positionSashes(): void {
    for (
      const sash of this.element.querySelectorAll<HTMLElement>(
        ":scope > .zeta-sash",
      )
    ) {
      const previousIndex = Number(sash.dataset.previousViewIndex);
      let position = 0;
      for (let index = 0; index <= previousIndex; index += 1) {
        const item = this.#items[index];
        if (item?.visible) position += item.size;
      }
      if (this.orientation === "horizontal") {
        sash.style.left = `${position}px`;
        sash.style.top = "0px";
        sash.style.height = `${this.#orthogonalSize}px`;
      } else {
        sash.style.left = "0px";
        sash.style.top = `${position}px`;
        sash.style.width = `${this.#orthogonalSize}px`;
      }
      const previous = this.#items[previousIndex];
      const next = this.#items.slice(previousIndex + 1).find(
        (item) => item.visible,
      );
      if (previous && next) {
        sash.setAttribute(
          "aria-valuemin",
          String(previous.view.minimumSize),
        );
        sash.setAttribute(
          "aria-valuemax",
          String(Math.min(
            previous.view.maximumSize,
            previous.size + next.size - next.view.minimumSize,
          )),
        );
        sash.setAttribute("aria-valuenow", String(previous.size));
      }
    }
  }

  #visibleItems(): ViewItem[] {
    return this.#items.filter((item) => item.visible);
  }

  #indexOf(view: ISplitViewView): number {
    const index = this.#items.findIndex((item) => item.view === view);
    if (index < 0) throw new Error("SplitView view is not registered");
    return index;
  }

  #item(index: number): ViewItem {
    const item = this.#items[index];
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
