import { WorkbenchPart } from "../../part.js";

/** The bottom status region for contextual workspace and connection information. */
export class StatusbarPart extends WorkbenchPart {
  readonly #message: HTMLSpanElement;

  constructor(message = "Ready") {
    super("statusbar");
    this.#message = document.createElement("span");
    this.#message.className = "zeta-statusbar-message";
    this.#message.textContent = message;
    this.contentElement.append(this.#message);
  }

  setMessage(message: string): void { this.#message.textContent = message; }
}
