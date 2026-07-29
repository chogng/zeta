import "./sessionpart.css";
import type {
  IWorkbenchSessionService,
} from "../../../services/sessions/common/sessionService.js";
import { WorkbenchPart } from "../../part.js";

/** The session selector and session-scoped navigation region. */
export class SessionPart extends WorkbenchPart {
  readonly #label: HTMLSpanElement;
  readonly #sessionService: IWorkbenchSessionService;

  override get minimumHeight(): number { return 36; }
  override get maximumHeight(): number { return 36; }

  constructor(
    ownerDocument: Document,
    sessionService: IWorkbenchSessionService,
  ) {
    super("session", ownerDocument);
    this.#sessionService = sessionService;
    this.#label = ownerDocument.createElement("span");
    this.#label.className = "zeta-session-label";
    this.contentElement.append(this.#label);
    this.own(sessionService.onDidChange(() => this.#render()));
    this.#render();
  }

  #render(): void {
    this.#label.removeAttribute("title");
    const active = this.#sessionService.active;
    if (active) {
      this.#label.textContent = active.session.title;
      return;
    }
    switch (this.#sessionService.state) {
      case "loading":
        this.#label.textContent = "Loading sessions…";
        break;
      case "creating":
        this.#label.textContent = "Creating session…";
        break;
      case "error":
        this.#label.textContent = "Session unavailable";
        this.#label.title =
          this.#sessionService.error ?? "Session unavailable";
        break;
      case "ready":
        this.#label.textContent = "No session";
        break;
    }
  }
}
