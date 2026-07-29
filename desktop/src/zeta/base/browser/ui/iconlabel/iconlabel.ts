import type { Icon } from "../../../common/icon.js";
import { DisposableOwner } from "../../../common/lifecycle.js";
import { appendIcon } from "../icon/icon.js";

/** Construction inputs for a semantic icon and text label. */
export interface IconLabelOptions {
  readonly label: string;
  readonly icon?: Icon;
  readonly renderIcon?: (container: HTMLSpanElement) => void;
  readonly ownerDocument?: Document;
  readonly reserveIconSpace?: boolean;
  readonly title?: string;
}

/**
 * Reusable label whose icon and text keep a stable, themeable DOM shape.
 */
export class IconLabel extends DisposableOwner {
  readonly element: HTMLSpanElement;
  readonly iconElement: HTMLSpanElement;
  readonly labelElement: HTMLSpanElement;

  constructor(options: IconLabelOptions) {
    super();
    if (options.icon && options.renderIcon) {
      throw new TypeError(
        "IconLabel accepts either a semantic icon or an icon renderer",
      );
    }
    const ownerDocument = options.ownerDocument ?? document;
    const element = ownerDocument.createElement("span");
    this.element = element;
    this.defer(() => element.remove());
    element.className = "zeta-icon-label";
    if (options.title) element.title = options.title;
    this.iconElement = ownerDocument.createElement("span");
    this.iconElement.className = "zeta-icon-label-icon";
    this.iconElement.setAttribute("aria-hidden", "true");
    this.iconElement.classList.toggle(
      "is-reserved",
      options.reserveIconSpace === true,
    );
    if (options.icon) {
      appendIcon(options.icon, this.iconElement);
    } else {
      options.renderIcon?.(this.iconElement);
    }
    this.labelElement = ownerDocument.createElement("span");
    this.labelElement.className = "zeta-icon-label-text";
    this.labelElement.textContent = options.label;
    element.append(this.iconElement, this.labelElement);
  }
}
