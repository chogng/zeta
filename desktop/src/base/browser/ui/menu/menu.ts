import { Button } from "../button/button.js";
import { DisposableOwner } from "../../../common/lifecycle.js";

export interface MenuItem { id: string; label: string; enabled?: boolean; run: () => void; }

/** A keyboard-focusable action menu suitable for a ContextView. */
export class Menu extends DisposableOwner {
  readonly element: HTMLDivElement;

  constructor(
    items: readonly MenuItem[],
    ownerDocument: Document = document,
  ) {
    super();
    const element = ownerDocument.createElement("div");
    this.element = element;
    this.defer(() => element.remove());
    element.className = "zeta-menu";
    element.setAttribute("role", "menu");
    for (const item of items) {
      const button = this.own(
        new Button({
          label: item.label,
          ownerDocument,
          enabled: item.enabled,
          onClick: item.run,
        }),
      );
      button.element.setAttribute("role", "menuitem");
      element.append(button.element);
    }
  }
}
