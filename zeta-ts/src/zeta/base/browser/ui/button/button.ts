import { addDisposableListener, h } from "../../dom.js";
import type { Icon } from "../../../common/icon.js";
import { DisposableOwner, DisposableSlot } from "../../../common/lifecycle.js";
import { setAriaAttribute } from "../aria/aria.js";
import type { AnchorPosition } from "../contextview/contextview.js";
import { getHoverDelegate, type IManagedHover } from "../hover/hoverDelegate.js";
import { IconLabel } from "../iconlabel/iconlabel.js";

/** Controls whether a button centers its complete content group or its text label. */
export type ButtonContentAlignment = "groupCentered" | "labelCentered";

export interface ButtonOptions {
  label: string;
  icon?: Icon;
  contentAlignment?: ButtonContentAlignment;
  title?: string;
  hoverGroupId?: string;
  hoverAnchorPosition?: AnchorPosition;
  enabled?: boolean;
  checked?: boolean;
  onClick?: () => void;
}

/** A semantic button with an explicit enabled state. */
export class Button extends DisposableOwner {
  readonly element: HTMLButtonElement;
  private readonly hover = this.own(new DisposableSlot<IManagedHover>());
  private readonly hoverGroupId: string | undefined;
  private readonly hoverAnchorPosition: AnchorPosition | undefined;

  constructor(container: HTMLElement, options: ButtonOptions) {
    super();
    const ownerDocument = container.ownerDocument;
    this.hoverGroupId = options.hoverGroupId;
    this.hoverAnchorPosition = options.hoverAnchorPosition;
    const element = h(ownerDocument, "button");
    this.element = element;
    this.defer(() => element.remove());
    element.className = "zeta-button";
    element.classList.toggle("label-centered", options.contentAlignment === "labelCentered");
    element.type = "button";
    const content = this.own(new IconLabel(element, {
      label: options.label,
      icon: options.icon,
    }));
    content.element.classList.add("zeta-button-content");
    content.labelElement.classList.add("zeta-button-label");
    container.append(element);
    this.setTitle(options.title);
    element.disabled = options.enabled === false;
    if (options.checked !== undefined) {
      this.checked = options.checked;
    }
    if (options.onClick) {
      this.own(addDisposableListener(element, "click", options.onClick));
    }
  }

  set enabled(value: boolean) { this.element.disabled = !value; }
  get enabled(): boolean { return !this.element.disabled; }

  set hidden(value: boolean) {
    this.element.hidden = value;
    this.element.classList.toggle("hidden", value);
  }

  get hidden(): boolean { return this.element.classList.contains("hidden"); }

  set checked(value: boolean) {
    this.element.classList.toggle("checked", value);
    setAriaAttribute(this.element, "pressed", value);
  }

  get checked(): boolean { return this.element.classList.contains("checked"); }

  setTitle(title: string | undefined): void {
    this.hover.clear();
    this.element.removeAttribute("title");
    if (!title) return;
    this.hover.replace(getHoverDelegate().setupHover({
      target: this.element,
      content: title,
      groupId: this.hoverGroupId,
      anchorPosition: this.hoverAnchorPosition,
    }));
  }
}
