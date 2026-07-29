import "./media/sessionsPart.css";
import type {
  IWorkbenchSessionService,
} from "../../../workbench/services/sessions/common/sessionService.js";
import { WorkbenchPart } from "../../../workbench/browser/part.js";

/**
 * Prototype surface for the Sessions-specific workbench.
 *
 * The regular Workbench does not mount this Part. The Sessions product layer
 * owns its future layout and can evolve this placeholder into the full
 * multi-session interface without adding a Session leaf beside EditorPart.
 */
export class SessionsPart extends WorkbenchPart {
  readonly #label: HTMLSpanElement;
  readonly #sessionService: IWorkbenchSessionService;

  override get minimumHeight(): number { return 36; }
  override get maximumHeight(): number { return 36; }

  constructor(
    ownerDocument: Document,
    sessionService: IWorkbenchSessionService,
  ) {
    super("sessions", ownerDocument);
    this.#sessionService = sessionService;
    this.#label = ownerDocument.createElement("span");
    this.#label.className = "zeta-sessions-label";
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
