import { WorkbenchPart } from "../../part.js";

/** The session selector and session-scoped navigation region. */
export class SessionPart extends WorkbenchPart {
  readonly #label: HTMLSpanElement;

  constructor(sessionName = "No session") {
    super("session");
    this.#label = document.createElement("span");
    this.#label.className = "zeta-session-label";
    this.#label.textContent = sessionName;
    this.contentElement.append(this.#label);
  }

  setSessionName(sessionName: string): void { this.#label.textContent = sessionName; }
}
