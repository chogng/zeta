import type { IViewPaneOptions } from "../../../browser/parts/views/viewPane.js";
import { ViewPane } from "../../../browser/parts/views/viewPane.js";

/** Projection point for review findings produced by an agent session. */
export class ScmAgentReviewViewPane extends ViewPane {
  constructor(options: IViewPaneOptions) {
    super(options);
    this.contentElement.classList.add("zeta-scm-secondary-pane");
    const empty = options.ownerDocument.createElement("p");
    empty.className = "zeta-scm-empty";
    empty.textContent = "No agent changes to review.";
    this.contentElement.append(empty);
  }
}
