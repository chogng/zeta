import { Button } from "../button/button.js";
import { Component } from "../common/component.js";

export interface MenuItem { id: string; label: string; enabled?: boolean; run: () => void; }

/** A keyboard-focusable action menu suitable for a ContextView. */
export class Menu extends Component<HTMLDivElement> {
  constructor(items: readonly MenuItem[]) {
    const element = document.createElement("div");
    element.className = "zeta-menu";
    element.setAttribute("role", "menu");
    super(element);
    for (const item of items) {
      const button = new Button({ label: item.label, enabled: item.enabled, onClick: item.run });
      button.element.setAttribute("role", "menuitem");
      this.element.append(button.element);
    }
  }
}
