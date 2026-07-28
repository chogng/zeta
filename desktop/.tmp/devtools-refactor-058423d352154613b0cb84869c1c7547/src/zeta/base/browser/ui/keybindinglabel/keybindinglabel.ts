import {
  getKeybindingLabel,
  getKeybindingLabelParts,
  KeybindingLabelStyle,
} from "../../../common/keybindingLabels.js";
import type {
  ResolvedKeybinding,
} from "../../../common/keybindings.js";
import { DisposableOwner } from "../../../common/lifecycle.js";
import { setAriaAttribute } from "../aria/aria.js";

export interface KeybindingLabelOptions {
  readonly keybinding: ResolvedKeybinding;
  readonly ownerDocument?: Document;
}

/** Presents a resolved keybinding without owning matching or dispatch policy. */
export class KeybindingLabel extends DisposableOwner {
  readonly element: HTMLSpanElement;
  #keybinding: ResolvedKeybinding;

  constructor(options: KeybindingLabelOptions) {
    super();
    const ownerDocument = options.ownerDocument ?? document;
    this.#keybinding = options.keybinding;
    this.element = ownerDocument.createElement("span");
    this.defer(() => this.element.remove());
    this.element.className = "zeta-keybinding-label";
    this.#render();
  }

  set keybinding(keybinding: ResolvedKeybinding) {
    this.#keybinding = keybinding;
    this.#render();
  }

  get keybinding(): ResolvedKeybinding {
    return this.#keybinding;
  }

  #render(): void {
    const ownerDocument = this.element.ownerDocument;
    const parts = getKeybindingLabelParts(this.#keybinding);
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
        this.#keybinding,
        KeybindingLabelStyle.Aria,
      ),
    );
  }
}
