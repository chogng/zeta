import "./sessionpart.css";
import { WorkbenchPart } from "../../part.js";

/** The session selector and session-scoped navigation region. */
export class SessionPart extends WorkbenchPart {
  readonly #label: HTMLSpanElement;

  override get minimumHeight(): number { return 36; }
  override get maximumHeight(): number { return 36; }

  constructor(
    ownerDocument: Document,
    sessionName = "No session",
  ) {
    super("session", ownerDocument);
    this.#label = ownerDocument.createElement("span");
    this.#label.className = "zeta-session-label";
    this.#label.textContent = sessionName;
    this.contentElement.append(this.#label);
  }

  setSessionName(sessionName: string): void { this.#label.textContent = sessionName; }
}
