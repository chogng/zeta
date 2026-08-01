import { addDisposableListener } from "../../../../base/browser/dom.js";
import type { GitCommitSummaryDto } from "../../../../../../generated/app-server/types.js";
import type { ZetaRendererApi } from "../../../../platform/app-server/common/renderer-api.js";
import type { IViewPaneOptions } from "../../../browser/parts/views/viewPane.js";
import { ViewPane } from "../../../browser/parts/views/viewPane.js";

/** Bounded recent repository history rendered as a compact commit graph. */
export class ScmGraphViewPane extends ViewPane {
  private readonly api: ZetaRendererApi;
  private readonly graphElement: HTMLDivElement;
  private disposed = false;

  constructor(options: IViewPaneOptions, api: ZetaRendererApi) {
    super(options);
    this.api = api;
    this.contentElement.classList.add("zeta-scm-secondary-pane");
    this.graphElement = options.ownerDocument.createElement("div");
    this.graphElement.className = "zeta-scm-graph";
    this.graphElement.setAttribute("role", "status");
    this.graphElement.setAttribute("aria-live", "polite");
    this.contentElement.append(this.graphElement);
    this.defer(() => {
      this.disposed = true;
    });
    void this.refresh();
  }

  private async refresh(): Promise<void> {
    this.graphElement.textContent = "Loading commit graph…";
    try {
      const { commits } = await this.api.git.history();
      if (this.disposed) return;
      this.renderCommits(commits);
    } catch (error) {
      if (this.disposed) return;
      const document = this.graphElement.ownerDocument;
      const message = document.createElement("p");
      message.className = "zeta-scm-empty";
      message.textContent = error instanceof Error ? error.message : String(error);
      const retry = document.createElement("button");
      retry.className = "zeta-scm-command";
      retry.type = "button";
      retry.textContent = "Retry";
      retry.setAttribute("aria-label", "Retry loading commit graph");
      this.own(addDisposableListener(retry, "click", () => void this.refresh()));
      this.graphElement.replaceChildren(message, retry);
    }
  }

  private renderCommits(commits: readonly GitCommitSummaryDto[]): void {
    if (commits.length === 0) {
      const empty = this.graphElement.ownerDocument.createElement("p");
      empty.className = "zeta-scm-empty";
      empty.textContent = "No commits yet.";
      this.graphElement.replaceChildren(empty);
      return;
    }
    const list = this.graphElement.ownerDocument.createElement("ol");
    list.className = "zeta-scm-graph-list";
    for (const commit of commits) list.append(this.renderCommit(commit));
    this.graphElement.replaceChildren(list);
  }

  private renderCommit(commit: GitCommitSummaryDto): HTMLLIElement {
    const document = this.graphElement.ownerDocument;
    const item = document.createElement("li");
    item.className = "zeta-scm-graph-commit";
    const node = document.createElement("span");
    node.className = "zeta-scm-graph-node";
    node.setAttribute("aria-hidden", "true");
    const details = document.createElement("span");
    details.className = "zeta-scm-graph-details";
    const subject = document.createElement("span");
    subject.className = "zeta-scm-graph-subject";
    subject.textContent = commit.subject;
    const metadata = document.createElement("span");
    metadata.className = "zeta-scm-graph-metadata";
    const date = new Date(commit.timestampSeconds * 1_000);
    metadata.textContent = `${commit.objectId.slice(0, 7)} · ${date.toLocaleDateString()}`;
    details.append(subject, metadata);
    item.append(node, details);
    return item;
  }
}
