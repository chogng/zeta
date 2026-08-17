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

const GRAPH_PAGE_SIZE = 50;
const GRAPH_LOAD_AHEAD_PX = 48;

/** Paged repository history rendered as a compact commit graph. */
export class ScmGraphViewPane extends ViewPane {
  private readonly gitService: IGitService;
  private readonly titleActions: ScmGraphTitleActions;
  private readonly graphElement: HTMLDivElement;
  private readonly commitHovers = this.own(new ResettableDisposableGroup());
  private readonly loadMoreControls = this.own(new ResettableDisposableGroup());
  private loadedGraph: GitGraph | undefined;
  private graphHead: GitHead | undefined;
  private graphRequestGeneration = 0;
  private nextGraphSkip = 0;
  private loadingMore = false;
  private loadMoreError: string | undefined;
  private disposed = false;

  constructor(options: IViewPaneOptions, gitService: IGitService, menuService: IMenuService, contextMenuService: IContextMenuService, contextKeyService: IContextKeyService, private readonly hoverService: IHoverService) {
    super({ ...options, headerActionsVisibility: "whenExpanded" });
    this.gitService = gitService;
    this.contentElement.classList.add("zeta-scm-secondary-pane");
    this.graphElement = options.ownerDocument.createElement("div");
    this.graphElement.className = "zeta-scm-graph";
    this.graphElement.setAttribute("role", "status");
    this.graphElement.setAttribute("aria-live", "polite");
    this.own(addDisposableListener(this.graphElement, "scroll", () => {
      if (this.graphElement.scrollTop + this.graphElement.clientHeight >= this.graphElement.scrollHeight - GRAPH_LOAD_AHEAD_PX) void this.loadMore();
    }));
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
    const generation = ++this.graphRequestGeneration;
    this.loadedGraph = undefined;
    this.graphHead = undefined;
    this.nextGraphSkip = 0;
    this.loadingMore = false;
    this.loadMoreError = undefined;
    this.commitHovers.clear();
    this.loadMoreControls.clear();
    this.graphElement.textContent = "Loading commit graph…";
    this.graphElement.setAttribute("aria-busy", "true");
    try {
      const [graph, status] = await Promise.all([
        this.gitService.graph({ limit: GRAPH_PAGE_SIZE, skip: 0 }),
        this.gitService.status(),
      ]);
      if (this.disposed || generation !== this.graphRequestGeneration) return;
      this.loadedGraph = graph;
      this.graphHead = status.head;
      this.nextGraphSkip = graph.commits.length;
      this.renderCommits(graph, status.head);
    } catch (error) {
      if (this.disposed || generation !== this.graphRequestGeneration) return;
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
      this.graphElement.setAttribute("aria-busy", "false");
    }
  }

  private renderCommits(graph: GitGraph, head: GitHead): void {
    this.commitHovers.clear();
    this.loadMoreControls.clear();
    const remotes = this.renderRemotes(graph);
    const children: HTMLElement[] = remotes ? [remotes] : [];
    if (graph.commits.length === 0) {
      const empty = this.graphElement.ownerDocument.createElement("p");
      empty.className = "zeta-scm-empty";
      empty.textContent = "No commits yet.";
      children.push(empty);
    } else {
      const referencesByObjectId = new Map<string, GitReference[]>();
      for (const reference of graph.references) {
        const references = referencesByObjectId.get(reference.objectId) ?? [];
        references.push(reference);
        referencesByObjectId.set(reference.objectId, references);
      }
      const list = this.graphElement.ownerDocument.createElement("ol");
      list.className = "zeta-scm-graph-list";
      for (const row of createScmGraphRows(graph.commits)) list.append(this.renderCommit(row.commit, head, referencesByObjectId.get(row.commit.objectId) ?? [], renderScmGraphRow(this.graphElement.ownerDocument, row, graphNodeKind(row.commit, head))));
      children.push(list);
    }
    if (graph.hasMore) children.push(this.renderLoadMoreControl());
    this.graphElement.replaceChildren(...children);
    this.graphElement.setAttribute("aria-busy", this.loadingMore ? "true" : "false");
  }

  private renderLoadMoreControl(): HTMLDivElement {
    const container = this.graphElement.ownerDocument.createElement("div");
    container.className = "zeta-scm-graph-load-more";
    if (this.loadMoreError) {
      const error = this.graphElement.ownerDocument.createElement("span");
      error.className = "zeta-scm-empty";
      error.textContent = this.loadMoreError;
      container.append(error);
    }
    const button = this.graphElement.ownerDocument.createElement("button");
    button.className = "zeta-scm-command";
    button.type = "button";
    button.disabled = this.loadingMore;
    button.textContent = this.loadingMore ? "Loading commit history…" : this.loadMoreError ? "Retry" : "Load more commits";
    button.setAttribute("aria-label", this.loadMoreError ? "Retry loading commit history" : "Load more commits");
    this.loadMoreControls.add(addDisposableListener(button, "click", () => void this.loadMore()));
    container.append(button);
    return container;
  }

  private async loadMore(): Promise<void> {
    const graph = this.loadedGraph;
    const head = this.graphHead;
    if (!graph || !head || !graph.hasMore || this.loadingMore) return;

    const generation = this.graphRequestGeneration;
    this.loadingMore = true;
    this.loadMoreError = undefined;
    this.renderCommits(graph, head);
    try {
      const page = await this.gitService.graph({ limit: GRAPH_PAGE_SIZE, skip: this.nextGraphSkip });
      if (this.disposed || generation !== this.graphRequestGeneration) return;
      const commits = [...graph.commits];
      const knownObjectIds = new Set(commits.map((commit) => commit.objectId));
      for (const commit of page.commits) {
        if (knownObjectIds.has(commit.objectId)) continue;
        knownObjectIds.add(commit.objectId);
        commits.push(commit);
      }
      const nextGraph: GitGraph = { ...page, commits };
      this.loadedGraph = nextGraph;
      this.nextGraphSkip += page.commits.length;
      this.loadingMore = false;
      this.renderCommits(nextGraph, head);
    } catch (error) {
      if (this.disposed || generation !== this.graphRequestGeneration) return;
      this.loadingMore = false;
      this.loadMoreError = gitErrorMessage(error);
      this.renderCommits(graph, head);
    }
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
