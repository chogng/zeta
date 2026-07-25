import { Component } from "../common/component.js";

/** A native-title based hover affordance for compact, accessible tooltips. */
export class Hover extends Component<HTMLSpanElement> {
  constructor(target: Element, text: string) {
    const element = document.createElement("span");
    element.className = "zeta-hover";
    element.title = text;
    super(element);
    element.append(target);
  }
}
