import { Component } from "../common/component.js";

/** A text label paired with an icon glyph or application-provided icon element. */
export class IconLabel extends Component<HTMLSpanElement> {
  constructor(label: string, icon?: string | Element) {
    const element = document.createElement("span");
    element.className = "zeta-icon-label";
    super(element);
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
