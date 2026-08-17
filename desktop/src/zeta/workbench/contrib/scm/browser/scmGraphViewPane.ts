import { addDisposableListener } from "../../../../base/browser/dom.js";
import { AnchorAlignment, AnchorAxisAlignment, AnchorPosition } from "../../../../base/browser/ui/contextview/contextview.js";
import { appendIcon } from "../../../../base/browser/ui/icon/icon.js";
import { lxiconsLibrary } from "../../../../base/common/lxiconsLibrary.js";
import { ResettableDisposableGroup } from "../../../../base/common/lifecycle.js";
import type { IMenuService } from "../../../../platform/actions/common/menuService.js";
import type { IContextKeyService } from "../../../../platform/contextkey/common/contextkey.js";
import type { IContextMenuService } from "../../../../platform/contextview/browser/contextMenu.js";
import type { IHoverService } from "../../../../platform/hover/common/hoverService.js";
import type { GitCommitSummary, GitGraph, GitHead, GitReference, GitRemoteProvider, IGitService } from "../../../services/git/common/gitService.js";
import type { IViewPaneOptions } from "../../../browser/parts/views/viewPane.js";
import { ViewPane } from "../../../browser/parts/views/viewPane.js";
import { createScmGraphRows, renderScmGraphRow, type ScmGraphNodeKind } from "./scmGraphRenderer.js";
import { ScmGraphTitleActions } from "./scmGraphTitleActions.js";
import { gitErrorMessage } from "./scmError.js";

/** Bounded recent repository history rendered as a compact commit graph. */
export class ScmGraphViewPane extends ViewPane {
  private readonly gitService: IGitService;
  private readonly titleActions: ScmGraphTitleActions;
  private readonly graphElement: HTMLDivElement;
  private readonly commitHovers = this.own(new ResettableDisposableGroup());
  private disposed = false;

  constructor(options: IViewPaneOptions, gitService: IGitService, menuService: IMenuService, contextMenuService: IContextMenuService, contextKeyService: IContextKeyService, private readonly hoverService: IHoverService) {
    super({ ...options, headerActionsVisibility: "whenExpanded" });
    this.gitService = gitService;
    this.contentElement.classList.add("zeta-scm-secondary-pane");
    this.graphElement = options.ownerDocument.createElement("div");
    this.graphElement.className = "zeta-scm-graph";
    this.graphElement.setAttribute("role", "status");
    this.graphElement.setAttribute("aria-live", "polite");
    this.contentElement.append(this.graphElement);
    this.titleActions = this.own(new ScmGraphTitleActions({
      ownerDocument: options.ownerDocument,
      gitService,
      menuService,
      contextMenuService,
      contextKeyService,
      refreshGraph: () => this.refresh(),
    }));
    this.headerActionsElement.append(this.titleActions.element);
    this.defer(() => {
      this.disposed = true;
    });
    void this.refresh();
  }

  private async refresh(): Promise<void> {
    this.commitHovers.clear();
    this.graphElement.textContent = "Loading commit graph…";
    try {
      const [graph, status] = await Promise.all([this.gitService.graph(), this.gitService.status()]);
      if (this.disposed) return;
      this.renderCommits(graph, status.head);
    } catch (error) {
      if (this.disposed) return;
      const document = this.graphElement.ownerDocument;
      const message = document.createElement("p");
      message.className = "zeta-scm-empty";
      message.textContent = gitErrorMessage(error);
      const retry = document.createElement("button");
      retry.className = "zeta-scm-command";
      retry.type = "button";
      retry.textContent = "Retry";
      retry.setAttribute("aria-label", "Retry loading commit graph");
      this.own(addDisposableListener(retry, "click", () => void this.refresh()));
      this.graphElement.replaceChildren(message, retry);
    }
  }

  private renderCommits(graph: GitGraph, head: GitHead): void {
    const remotes = this.renderRemotes(graph);
    if (graph.commits.length === 0) {
      const empty = this.graphElement.ownerDocument.createElement("p");
      empty.className = "zeta-scm-empty";
      empty.textContent = "No commits yet.";
      this.graphElement.replaceChildren(...(remotes ? [remotes, empty] : [empty]));
      return;
    }
    const referencesByObjectId = new Map<string, GitReference[]>();
    for (const reference of graph.references) {
      const references = referencesByObjectId.get(reference.objectId) ?? [];
      references.push(reference);
      referencesByObjectId.set(reference.objectId, references);
    }
    const list = this.graphElement.ownerDocument.createElement("ol");
    list.className = "zeta-scm-graph-list";
    for (const row of createScmGraphRows(graph.commits)) list.append(this.renderCommit(row.commit, head, referencesByObjectId.get(row.commit.objectId) ?? [], renderScmGraphRow(this.graphElement.ownerDocument, row, graphNodeKind(row.commit, head))));
    this.graphElement.replaceChildren(...(remotes ? [remotes, list] : [list]));
  }

  private renderCommit(commit: GitCommitSummary, head: GitHead, references: readonly GitReference[], graph: SVGSVGElement): HTMLLIElement {
    const document = this.graphElement.ownerDocument;
    const item = document.createElement("li");
    item.className = "zeta-scm-graph-commit";
    const current = headObjectId(head) === commit.objectId;
    const merge = commit.parentObjectIds.length > 1;
    item.classList.toggle("current", current);
    item.classList.toggle("head", current);
    item.classList.toggle("merge", merge);
    item.classList.toggle("commit", !current && !merge);
    if (current) item.setAttribute("aria-current", "true");
    this.commitHovers.add(this.hoverService.setupHover({
      target: item,
      content: () => this.renderCommitHover(commit),
      groupId: "scm.history.items",
      anchorAlignment: AnchorAlignment.Left,
      anchorAxisAlignment: AnchorAxisAlignment.Horizontal,
      anchorPosition: AnchorPosition.Below,
      gap: 8,
    }));
    const details = document.createElement("span");
    details.className = "zeta-scm-graph-details";
    const subject = document.createElement("span");
    subject.className = "zeta-scm-graph-subject";
    subject.textContent = commit.subject;
    details.append(subject);
    if (current) details.append(this.renderHeadLabel(head));
    const visibleReferences = references.filter((reference) => !(current && reference.kind === "localBranch" && reference.current));
    if (visibleReferences.length > 0) details.append(this.renderReferenceLabels(visibleReferences));
    const metadata = document.createElement("span");
    metadata.className = "zeta-scm-graph-metadata";
    const date = new Date(commit.timestampSeconds * 1_000);
    metadata.textContent = `${commit.objectId.slice(0, 7)} · ${date.toLocaleDateString(undefined, { month: "short", day: "numeric" })}`;
    item.append(graph, details, metadata);
    return item;
  }

  private renderReferenceLabels(references: readonly GitReference[]): HTMLSpanElement {
    const container = this.graphElement.ownerDocument.createElement("span");
    container.className = "zeta-scm-graph-refs";
    container.setAttribute("aria-label", "Git references");
    for (const reference of references) {
      const label = this.graphElement.ownerDocument.createElement("span");
      label.className = `zeta-scm-graph-ref ${reference.kind === "remoteBranch" ? "remote" : "local"}`;
      label.textContent = reference.name;
      label.title = reference.kind === "remoteBranch" ? `Fetched remote branch ${reference.name}` : `Local branch ${reference.name}`;
      container.append(label);
    }
    return container;
  }

  private renderRemotes(graph: GitGraph): HTMLDivElement | undefined {
    if (graph.remotes.length === 0) return undefined;
    const container = this.graphElement.ownerDocument.createElement("div");
    container.className = "zeta-scm-graph-remotes";
    container.setAttribute("aria-label", "Git remotes");
    for (const remote of graph.remotes) {
      const label = this.graphElement.ownerDocument.createElement("span");
      label.className = "zeta-scm-graph-remote";
      label.textContent = remote.identity ? `${remoteLabel(remote.identity.provider)} · ${remote.identity.owner}/${remote.identity.repository} · ${remote.name}` : remote.name;
      label.title = remote.identity ? `${remote.identity.host}/${remote.identity.owner}/${remote.identity.repository}` : `Git remote ${remote.name}`;
      container.append(label);
    }
    return container;
  }

  private renderHeadLabel(head: GitHead): HTMLSpanElement {
    const label = this.graphElement.ownerDocument.createElement("span");
    label.className = "zeta-scm-graph-head";
    appendIcon(head.type === "branch" ? lxiconsLibrary.gitBranch : lxiconsLibrary.gitCommit, label);
    const text = this.graphElement.ownerDocument.createElement("span");
    text.textContent = head.type === "branch" ? head.name : head.type === "detached" ? head.objectId.slice(0, 7) : head.name;
    label.append(text);
    if (head.type === "branch" && head.upstream) label.title = `${head.name} tracks ${head.upstream.name}`;
    return label;
  }

  private renderCommitHover(commit: GitCommitSummary): HTMLDivElement {
    const document = this.graphElement.ownerDocument;
    const hover = document.createElement("div");
    hover.className = "zeta-scm-graph-hover";
    const subject = document.createElement("div");
    subject.className = "zeta-scm-graph-hover-subject";
    subject.textContent = commit.subject;
    const metadata = document.createElement("div");
    metadata.className = "zeta-scm-graph-hover-metadata";
    metadata.textContent = `${commit.objectId} · ${new Date(commit.timestampSeconds * 1_000).toLocaleString()}`;
    hover.append(subject, metadata);
    return hover;
  }
}

function headObjectId(head: GitHead): string | undefined {
  return head.type === "unborn" ? undefined : head.objectId;
}

function graphNodeKind(commit: GitCommitSummary, head: GitHead): ScmGraphNodeKind {
  if (headObjectId(head) === commit.objectId) return "head";
  return commit.parentObjectIds.length > 1 ? "merge" : "commit";
}

function remoteLabel(provider: GitRemoteProvider): string {
  switch (provider) {
    case "github": return "GitHub";
    case "gitlab": return "GitLab";
    case "bitbucket": return "Bitbucket";
    case "other": return "Remote";
  }
}
