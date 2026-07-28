import { Emitter } from "../../../common/event.js";
import {
  DisposableOwner,
} from "../../../common/lifecycle.js";
import {
  addDisposableListener,
  stopEvent,
} from "../../dom.js";
import {
  AnchorAlignment,
  AnchorAxisAlignment,
  AnchorPosition,
  ContextView,
  ContextViewFocusRestore,
  type ContextViewHideReason,
  type IContextViewProvider,
} from "../contextview/contextview.js";

export type DropdownContent = HTMLElement | (() => HTMLElement);

export interface DropdownOptions {
  readonly label: string;
  readonly content: DropdownContent;
  readonly ownerDocument?: Document;
  readonly ariaLabel?: string;
  readonly anchorAlignment?: AnchorAlignment;
  readonly anchorPosition?: AnchorPosition;
  readonly anchorAxisAlignment?: AnchorAxisAlignment;
  readonly gap?: number;
  readonly contextViewProvider?: IContextViewProvider;
}

export interface DropdownVisibilityChangeEvent {
  readonly visible: boolean;
  readonly reason?: ContextViewHideReason;
}

let dropdownId = 0;

/** A button that owns the visibility lifecycle of an anchored popup. */
export class Dropdown extends DisposableOwner {
  readonly element: HTMLDivElement;
  readonly button: HTMLButtonElement;
  readonly #label: HTMLSpanElement;
  readonly #content: DropdownContent;
  readonly #contextView: IContextViewProvider;
  readonly #onDidChangeVisibility = this.own(
    new Emitter<DropdownVisibilityChangeEvent>(),
  );
  readonly onDidChangeVisibility = this.#onDidChangeVisibility.event;
  readonly #options: DropdownOptions;
  #visible = false;

  constructor(options: DropdownOptions) {
    super();
    this.#options = options;
    this.#content = options.content;
    const ownerDocument = options.ownerDocument ?? document;
    const element = ownerDocument.createElement("div");
    this.element = element;
    this.defer(() => element.remove());
    element.className = "zeta-dropdown";

    const button = ownerDocument.createElement("button");
    this.button = button;
    button.className = "zeta-dropdown-button";
    button.type = "button";
    button.setAttribute("aria-haspopup", "true");
    button.setAttribute("aria-expanded", "false");
    if (options.ariaLabel) {
      button.setAttribute("aria-label", options.ariaLabel);
    }

    const label = ownerDocument.createElement("span");
    this.#label = label;
    label.className = "zeta-dropdown-label";
    label.textContent = options.label;
    const indicator = ownerDocument.createElement("span");
    indicator.className = "zeta-dropdown-indicator";
    indicator.textContent = "\u25be";
    indicator.setAttribute("aria-hidden", "true");
    button.append(label, indicator);
    element.append(button);

    this.#contextView = options.contextViewProvider ??
      this.own(new ContextView(ownerDocument));
    this.defer(() => this.hide());
    this.own(addDisposableListener(button, "click", () => this.toggle()));
    this.own(addDisposableListener(button, "keydown", (event) => {
      if (
        event.key !== "ArrowDown" &&
        event.key !== "ArrowUp" &&
        event.key !== "Enter" &&
        event.key !== " "
      ) {
        return;
      }
      stopEvent(event);
      this.show();
    }));
  }

  get visible(): boolean {
    return this.#visible;
  }

  get enabled(): boolean {
    return !this.button.disabled;
  }

  set enabled(value: boolean) {
    this.button.disabled = !value;
    if (!value) this.hide();
  }

  setLabel(label: string): void {
    this.#label.textContent = label;
  }

  show(): void {
    if (this.#visible || !this.enabled) return;
    const content = typeof this.#content === "function"
      ? this.#content()
      : this.#content;
    content.classList.add("zeta-dropdown-content");
    if (!content.id) {
      dropdownId += 1;
      content.id = `zeta-dropdown-content-${dropdownId}`;
    }
    this.button.setAttribute("aria-controls", content.id);
    const shown = this.#contextView.show({
      anchor: this.button,
      content,
      anchorAlignment: this.#options.anchorAlignment,
      anchorPosition: this.#options.anchorPosition,
      anchorAxisAlignment: this.#options.anchorAxisAlignment,
      gap: this.#options.gap,
      focusRestore: ContextViewFocusRestore.Previous,
      onHide: (reason) => this.#didHide(reason),
    });
    if (!shown) return;
    this.#visible = true;
    this.element.classList.add("zeta-dropdown-open");
    this.button.setAttribute("aria-expanded", "true");
    this.#onDidChangeVisibility.fire({ visible: true });
  }

  hide(): void {
    if (!this.#visible) return;
    this.#contextView.hide();
  }

  toggle(): void {
    if (this.#visible) this.hide();
    else this.show();
  }

  focus(): void {
    this.button.focus();
  }

  #didHide(reason: ContextViewHideReason): void {
    if (!this.#visible) return;
    this.#visible = false;
    this.element.classList.remove("zeta-dropdown-open");
    this.button.setAttribute("aria-expanded", "false");
    this.#onDidChangeVisibility.fire({ visible: false, reason });
  }
}
