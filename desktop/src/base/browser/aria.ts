import {
  DisposableOwner,
  DisposableSlot,
  type IDisposable,
} from "../common/lifecycle.js";
import { scheduleAtNextAnimationFrame } from "./scheduler.js";
import { getWindow } from "./window.js";

export type AriaLivePriority = "polite" | "assertive";

/** Owns hidden ARIA live regions for one document. */
export class AriaLiveRegion extends DisposableOwner {
  readonly #root: HTMLDivElement;
  readonly #polite: HTMLDivElement;
  readonly #assertive: HTMLDivElement;
  readonly #pending = this.own(new DisposableSlot<IDisposable>());

  constructor(ownerDocument: Document) {
    super();
    this.#root = ownerDocument.createElement("div");
    this.#root.className = "zeta-aria-live";
    Object.assign(this.#root.style, {
      position: "fixed",
      width: "1px",
      height: "1px",
      overflow: "hidden",
      clipPath: "inset(50%)",
      whiteSpace: "nowrap",
    });
    this.#polite = this.#createRegion(ownerDocument, "polite");
    this.#assertive = this.#createRegion(ownerDocument, "assertive");
    this.#root.append(this.#polite, this.#assertive);
    ownerDocument.body.append(this.#root);
    this.defer(() => this.#root.remove());
  }

  announce(
    message: string,
    priority: AriaLivePriority = "polite",
  ): void {
    const region = priority === "assertive"
      ? this.#assertive
      : this.#polite;
    region.textContent = "";
    this.#pending.replace(scheduleAtNextAnimationFrame(
      getWindow(region),
      () => {
        this.#pending.clear();
        region.textContent = message;
      },
    ));
  }

  clear(): void {
    this.#pending.clear();
    this.#polite.textContent = "";
    this.#assertive.textContent = "";
  }

  #createRegion(
    ownerDocument: Document,
    priority: AriaLivePriority,
  ): HTMLDivElement {
    const region = ownerDocument.createElement("div");
    region.setAttribute("aria-live", priority);
    region.setAttribute("aria-atomic", "true");
    return region;
  }
}
