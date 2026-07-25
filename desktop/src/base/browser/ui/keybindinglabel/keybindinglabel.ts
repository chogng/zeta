import { Component } from "../common/component.js";

/** Renders keyboard shortcuts as individually styled key tokens. */
export class KeybindingLabel extends Component<HTMLSpanElement> {
  constructor(keys: readonly string[]) {
    const element = document.createElement("span");
    element.className = "zeta-keybinding-label";
    super(element);
    element.append(...keys.map((key) => {
      const token = document.createElement("kbd");
      token.textContent = key;
      return token;
    }));
  }
}
