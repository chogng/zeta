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
import { setAriaAttribute } from "../aria/aria.js";

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
  private readonly label: HTMLSpanElement;
  private readonly content: DropdownContent;
  private readonly contextView: IContextViewProvider;
  private readonly _onDidChangeVisibility = this.own(
    new Emitter<DropdownVisibilityChangeEvent>(),
  );
  readonly onDidChangeVisibility = this._onDidChangeVisibility.event;
  private readonly options: DropdownOptions;
  private _visible = false;

  constructor(options: DropdownOptions) {
    super();
    this.options = options;
    this.content = options.content;
    const ownerDocument = options.ownerDocument ?? document;
    const element = ownerDocument.createElement("div");
    this.element = element;
    this.defer(() => element.remove());
    element.className = "zeta-dropdown";

    const button = ownerDocument.createElement("button");
    this.button = button;
    button.className = "zeta-dropdown-button";
    button.type = "button";
    setAriaAttribute(button, "haspopup", true);
    setAriaAttribute(button, "expanded", false);
    if (options.ariaLabel) {
      setAriaAttribute(button, "label", options.ariaLabel);
    }

    const label = ownerDocument.createElement("span");
    this.label = label;
    label.className = "zeta-dropdown-label";
    label.textContent = options.label;
    const indicator = ownerDocument.createElement("span");
    indicator.className = "zeta-dropdown-indicator";
    indicator.textContent = "\u25be";
    setAriaAttribute(indicator, "hidden", true);
    button.append(label, indicator);
    element.append(button);

    this.contextView = options.contextViewProvider ??
      this.own(new ContextView(ownerDocument.body));
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
    return this._visible;
  }

  get enabled(): boolean {
    return !this.button.disabled;
  }

  set enabled(value: boolean) {
    this.button.disabled = !value;
    if (!value) this.hide();
  }

  setLabel(label: string): void {
    this.label.textContent = label;
  }

  show(): void {
    if (this._visible || !this.enabled) return;
    const content = typeof this.content === "function"
      ? this.content()
      : this.content;
    content.classList.add("zeta-dropdown-content");
    if (!content.id) {
      dropdownId += 1;
      content.id = `zeta-dropdown-content-${dropdownId}`;
    }
    setAriaAttribute(this.button, "controls", content.id);
    const shown = this.contextView.show({
      anchor: this.button,
      content,
      anchorAlignment: this.options.anchorAlignment,
      anchorPosition: this.options.anchorPosition,
      anchorAxisAlignment: this.options.anchorAxisAlignment,
      gap: this.options.gap,
      focusRestore: ContextViewFocusRestore.Previous,
      onHide: (reason) => this.didHide(reason),
    });
    if (!shown) return;
    this._visible = true;
    this.element.classList.add("zeta-dropdown-open");
    setAriaAttribute(this.button, "expanded", true);
    this._onDidChangeVisibility.fire({ visible: true });
  }

  hide(): void {
    if (!this._visible) return;
    this.contextView.hide();
  }

  toggle(): void {
    if (this._visible) this.hide();
    else this.show();
  }

  focus(): void {
    this.button.focus();
  }

  private didHide(reason: ContextViewHideReason): void {
    if (!this._visible) return;
    this._visible = false;
    this.element.classList.remove("zeta-dropdown-open");
    setAriaAttribute(this.button, "expanded", false);
    this._onDidChangeVisibility.fire({ visible: false, reason });
  }
}
