import { Button } from "../../../../base/browser/ui/button/button.js";
import { lxiconsLibrary } from "../../../../base/common/lxiconsLibrary.js";
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
    const findIssues = this.own(new Button({
      label: "Find Issues",
      icon: lxiconsLibrary.codeReview,
      contentAlignment: "labelCentered",
      ownerDocument: options.ownerDocument,
      title: "Find Issues",
    }));
    findIssues.element.classList.add("zeta-scm-find-issues");
    this.contentElement.append(empty, findIssues.element);
  }
}
