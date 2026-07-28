import { DisposableOwner } from "../../../common/lifecycle.js";

/** A text label paired with an icon glyph or application-provided icon element. */
export class IconLabel extends DisposableOwner {
  readonly element: HTMLSpanElement;

  constructor(label: string, icon?: string | Element) {
    super();
    const element = document.createElement("span");
    this.element = element;
    this.defer(() => element.remove());
    element.className = "zeta-icon-label";
    if (icon) {
      const iconElement = typeof icon === "string" ? document.createElement("span") : icon;
      if (typeof icon === "string") iconElement.textContent = icon;
      iconElement.classList.add("zeta-icon-label-icon");
      element.append(iconElement);
    }
    const text = document.createElement("span");
    text.textContent = label;
    element.append(text);
  }
}
