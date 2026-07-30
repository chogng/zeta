import { getKeybindingLabel, getKeybindingLabelParts, KeybindingLabelStyle } from "../../../common/keybindingLabels.js";
import type { ResolvedKeybinding } from "../../../common/keybindings.js";
import { DisposableOwner } from "../../../common/lifecycle.js";
import { setAriaAttribute } from "../aria/aria.js";

export interface KeybindingLabelOptions {
  readonly keybinding: ResolvedKeybinding;
  readonly ownerDocument?: Document;
  readonly presentation?: KeybindingLabelPresentation;
}

/** Component-owned visual treatment for a rendered keybinding. */
export type KeybindingLabelPresentation = "plain" | "keycap";

/** Presents a resolved keybinding without owning matching or dispatch policy. */
export class KeybindingLabel extends DisposableOwner {
  readonly element: HTMLSpanElement;
  private _keybinding: ResolvedKeybinding;

  constructor(options: KeybindingLabelOptions) {
    super();
    const ownerDocument = options.ownerDocument ?? document;
    this._keybinding = options.keybinding;
    this.element = ownerDocument.createElement("span");
    this.defer(() => this.element.remove());
    this.element.className = `zeta-keybinding-label zeta-keybinding-label-${options.presentation ?? "plain"}`;
    this.render();
  }

  set keybinding(keybinding: ResolvedKeybinding) {
    this._keybinding = keybinding;
    this.render();
  }

  get keybinding(): ResolvedKeybinding {
    return this._keybinding;
  }

  private render(): void {
    const ownerDocument = this.element.ownerDocument;
    const parts = getKeybindingLabelParts(this._keybinding);
    this.element.replaceChildren(...parts.map((part) => {
      const token = ownerDocument.createElement("kbd");
      token.textContent = part.label;
      setAriaAttribute(token, "label", part.ariaLabel);
      return token;
    }));
    setAriaAttribute(
      this.element,
      "label",
      getKeybindingLabel(
        this._keybinding,
        KeybindingLabelStyle.Aria,
      ),
    );
  }
}
