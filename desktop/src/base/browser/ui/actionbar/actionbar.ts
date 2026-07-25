import { Button, type ButtonOptions } from "../button/button.js";
import { Component } from "../common/component.js";

/** A horizontal group of related command buttons. */
export class ActionBar extends Component<HTMLDivElement> {
  constructor(actions: readonly ButtonOptions[] = []) {
    const element = document.createElement("div");
    element.className = "zeta-action-bar";
    element.setAttribute("role", "toolbar");
    super(element);
    for (const action of actions) this.add(action);
  }

  add(action: ButtonOptions): Button {
    const button = new Button(action);
    this.element.append(button.element);
    return button;
  }
}
