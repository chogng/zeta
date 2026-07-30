import { Emitter } from "../../../common/event.js";
import {
  DisposableOwner,
  DisposableSlot,
  ResettableDisposableGroup,
  type IDisposable,
  toDisposable,
} from "../../../common/lifecycle.js";
import {
  addDisposableListener,
  isNode,
} from "../../dom.js";
import { getWindow } from "../../window.js";
import {
  getAriaAttribute,
  setAriaAttribute,
} from "../aria/aria.js";
import {
  AnchorAlignment,
  AnchorPosition,
  ContextView,
  type ContextViewHideReason,
  type IContextViewProvider,
} from "../contextview/contextview.js";

export type HoverContent =
  | string
  | HTMLElement
  | (() => string | HTMLElement);

export interface HoverOptions {
  readonly target: HTMLElement;
  readonly content: HoverContent;
  readonly delayMs?: number;
  readonly anchorAlignment?: AnchorAlignment;
  readonly anchorPosition?: AnchorPosition;
  readonly gap?: number;
  readonly contextViewProvider?: IContextViewProvider;
}

let hoverId = 0;

/** A managed, accessible tooltip hosted in a ContextView. */
export class Hover extends DisposableOwner {
  readonly element: HTMLElement;
  private readonly contextView: IContextViewProvider;
  private readonly showTimer = this.own(new DisposableSlot<IDisposable>());
  private readonly hideTimer = this.own(new DisposableSlot<IDisposable>());
  private readonly tooltipListeners = this.own(new ResettableDisposableGroup());
  private readonly _onDidShow = this.own(new Emitter<void>());
  private readonly _onDidHide = this.own(new Emitter<void>());
  readonly onDidShow = this._onDidShow.event;
  readonly onDidHide = this._onDidHide.event;
  private readonly delayMs: number;
  private readonly anchorAlignment: AnchorAlignment;
  private readonly anchorPosition: AnchorPosition;
  private readonly gap: number;
  private content: HoverContent;
  private tooltip: HTMLDivElement | undefined;
  private previousTitle: string | undefined;
  private previousDescription: string | undefined;
  private descriptionApplied = false;
  private _visible = false;

  constructor(options: HoverOptions) {
    super();
    const target = options.target;
    this.element = target;
    this.content = options.content;
    this.delayMs = Math.max(0, options.delayMs ?? 300);
    this.anchorAlignment = options.anchorAlignment ??
      AnchorAlignment.Left;
    this.anchorPosition = options.anchorPosition ??
      AnchorPosition.Above;
    this.gap = Math.max(0, options.gap ?? 6);
    this.contextView = options.contextViewProvider ??
      this.own(new ContextView(target.ownerDocument.body));

    const title = target.getAttribute("title");
    if (title !== null) {
      this.previousTitle = title;
      target.removeAttribute("title");
    }
    this.defer(() => {
      this.restoreDescription();
      if (this.previousTitle !== undefined) {
        target.setAttribute("title", this.previousTitle);
      }
    });
    this.defer(() => this.hide());

    this.own(addDisposableListener(target, "pointerenter", () => {
      this.hideTimer.clear();
      this.scheduleShow();
    }));
    this.own(addDisposableListener(target, "pointerleave", (event) => {
      if (this.isInsideHover(event.relatedTarget)) return;
      this.scheduleHide();
    }));
    this.own(addDisposableListener(target, "focusin", () => this.show()));
    this.own(addDisposableListener(target, "focusout", (event) => {
      if (this.isInsideHover(event.relatedTarget)) return;
      this.scheduleHide();
    }));
  }

  get visible(): boolean {
    return this._visible;
  }

  show(): void {
    this.showTimer.clear();
    this.hideTimer.clear();
    if (this.visible) return;
    const ownerDocument = this.element.ownerDocument;
    const tooltip = ownerDocument.createElement("div");
    hoverId += 1;
    tooltip.id = `zeta-hover-${hoverId}`;
    tooltip.className = "zeta-hover";
    tooltip.setAttribute("role", "tooltip");
    this.renderContent(tooltip);
    this.tooltipListeners.clear();
    this.tooltipListeners.add(addDisposableListener(
      tooltip,
      "pointerenter",
      () => this.hideTimer.clear(),
    ));
    this.tooltipListeners.add(addDisposableListener(
      tooltip,
      "pointerleave",
      (event) => {
        if (
          isNode(event.relatedTarget) &&
          this.element.contains(event.relatedTarget)
        ) {
          return;
        }
        this.scheduleHide();
      },
    ));
    this.tooltip = tooltip;
    this.applyDescription(tooltip.id);
    const shown = this.contextView.show({
      anchor: this.element,
      content: tooltip,
      anchorAlignment: this.anchorAlignment,
      anchorPosition: this.anchorPosition,
      gap: this.gap,
      onHide: (reason) => this.didHide(reason),
    });
    if (!shown) return;
    this._visible = true;
    this._onDidShow.fire();
  }

  hide(): void {
    this.showTimer.clear();
    this.hideTimer.clear();
    if (!this._visible) return;
    this.contextView.hide();
  }

  update(content: HoverContent): void {
    this.content = content;
    if (!this._visible || !this.tooltip) return;
    this.renderContent(this.tooltip);
    this.contextView.layout();
  }

  private scheduleShow(): void {
    if (this.visible || this.showTimer.value) return;
    this.showTimer.replace(windowTimeout(
      getWindow(this.element),
      () => {
        this.showTimer.clear();
        this.show();
      },
      this.delayMs,
    ));
  }

  private scheduleHide(): void {
    this.showTimer.clear();
    if (!this.visible || this.hideTimer.value) return;
    this.hideTimer.replace(windowTimeout(
      getWindow(this.element),
      () => {
        this.hideTimer.clear();
        this.hide();
      },
      80,
    ));
  }

  private renderContent(container: HTMLElement): void {
    const content = typeof this.content === "function"
      ? this.content()
      : this.content;
    container.replaceChildren();
    if (typeof content === "string") {
      container.textContent = content;
    } else {
      container.append(content);
    }
  }

  private isInsideHover(candidate: EventTarget | null): boolean {
    return isNode(candidate) && Boolean(this.tooltip?.contains(candidate));
  }

  private applyDescription(id: string): void {
    this.previousDescription = getAriaAttribute(
      this.element,
      "describedby",
    );
    this.descriptionApplied = true;
    const ids = new Set(
      this.previousDescription?.split(/\s+/).filter(Boolean) ?? [],
    );
    ids.add(id);
    setAriaAttribute(this.element, "describedby", [...ids].join(" "));
  }

  private restoreDescription(): void {
    if (!this.descriptionApplied) return;
    if (this.previousDescription === undefined) {
      setAriaAttribute(this.element, "describedby", undefined);
    } else {
      setAriaAttribute(
        this.element,
        "describedby",
        this.previousDescription,
      );
    }
    this.previousDescription = undefined;
    this.descriptionApplied = false;
  }

  private didHide(_reason: ContextViewHideReason): void {
    const wasVisible = this._visible;
    this._visible = false;
    this.tooltip = undefined;
    this.tooltipListeners.clear();
    this.restoreDescription();
    if (wasVisible) this._onDidHide.fire();
  }
}

function windowTimeout(
  targetWindow: Window,
  callback: () => void,
  delayMs: number,
): IDisposable {
  const handle = targetWindow.setTimeout(callback, delayMs);
  return toDisposable(() => targetWindow.clearTimeout(handle));
}
