import { addDisposableListener } from "../../../../base/browser/dom.js";
import { AnchorAlignment, AnchorAxisAlignment, AnchorPosition } from "../../../../base/browser/ui/contextview/contextview.js";
import { appendIcon } from "../../../../base/browser/ui/icon/icon.js";
import { lxiconsLibrary } from "../../../../base/common/lxiconsLibrary.js";
import { ResettableDisposableGroup } from "../../../../base/common/lifecycle.js";
import type { IMenuService } from "../../../../platform/actions/common/menuService.js";
import type { IContextKeyService } from "../../../../platform/contextkey/common/contextkey.js";
import type { IContextMenuService } from "../../../../platform/contextview/browser/contextMenu.js";
import type { IHoverService } from "../../../../platform/hover/common/hoverService.js";
import type { GitCommitSummary, GraphPage, GitHead, GitReference, GitRemoteProvider, IGitService } from "../../../services/git/common/gitService.js";
import type { IViewPaneOptions } from "../../../browser/parts/views/viewPane.js";
import { ViewPane } from "../../../browser/parts/views/viewPane.js";
import { createRows, GraphRowHeight, renderRow, type GraphNodeKind, type GraphRow, type GraphState } from "./scmGraphRenderer.js";
import { ScmGraphTitleActions } from "./scmGraphTitleActions.js";
import { gitErrorMessage } from "./scmError.js";

const PageSize = 50;
const LoadAhead = 48;
const Overscan = 8;

/** Paged repository history rendered as a compact commit graph. */
export class ScmGraphViewPane extends ViewPane {
  private readonly gitService: IGitService;
  private readonly actions: ScmGraphTitleActions;
  private readonly graphElement: HTMLDivElement;
  private readonly hovers = this.own(new ResettableDisposableGroup());
  private readonly more = this.own(new ResettableDisposableGroup());
  private page: GraphPage | undefined;
  private head: GitHead | undefined;
  private generation = 0;
  private cursor: string | undefined;
  private loading = false;
  private moreError: string | undefined;
  private rows: readonly GraphRow[] = [];
  private graphState: GraphState = { lanes: [], nextColor: 0 };
  private list: HTMLOListElement | undefined;
  private readonly refs = new Map<string, readonly GitReference[]>();
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
      this.renderRows();
      if (this.graphElement.scrollTop + this.graphElement.clientHeight >= this.graphElement.scrollHeight - LoadAhead) void this.loadMore();
    }));
    this.contentElement.append(this.graphElement);
    this.actions = this.own(new ScmGraphTitleActions({
      ownerDocument: options.ownerDocument,
      gitService,
      menuService,
      contextMenuService,
      contextKeyService,
      refreshGraph: () => this.refresh(),
    }));
    this.headerActionsElement.append(this.actions.element);
    this.defer(() => {
      this.disposed = true;
    });
    void this.refresh();
  }

  private async refresh(): Promise<void> {
    const generation = ++this.generation;
    this.page = undefined;
    this.head = undefined;
    this.cursor = undefined;
    this.loading = false;
    this.moreError = undefined;
    this.rows = [];
    this.graphState = { lanes: [], nextColor: 0 };
    this.list = undefined;
    this.refs.clear();
    this.hovers.clear();
    this.more.clear();
    this.graphElement.textContent = "Loading commit graph…";
    this.graphElement.setAttribute("aria-busy", "true");
    try {
      const [graph, status] = await Promise.all([
        this.gitService.graph({ limit: PageSize }),
        this.gitService.status(),
      ]);
      if (this.disposed || generation !== this.generation) return;
      this.page = graph;
      this.head = status.head;
      this.cursor = graph.nextCursor;
      this.renderGraph(graph, status.head);
    } catch (error) {
      if (this.disposed || generation !== this.generation) return;
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

  private renderGraph(graph: GraphPage, head: GitHead): void {
    this.hovers.clear();
    this.more.clear();
    this.rows = [];
    this.graphState = { lanes: [], nextColor: 0 };
    this.refs.clear();
    for (const reference of graph.references) {
      const references = this.refs.get(reference.objectId) ?? [];
      this.refs.set(reference.objectId, [...references, reference]);
    }
    const batch = createRows(graph.commits);
    this.rows = batch.rows;
    this.graphState = batch.state;
    const remotes = this.renderRemotes(graph);
    const children: HTMLElement[] = remotes ? [remotes] : [];
    this.list = undefined;
    if (graph.commits.length === 0) {
      const empty = this.graphElement.ownerDocument.createElement("p");
      empty.className = "zeta-scm-empty";
      empty.textContent = "No commits yet.";
      children.push(empty);
    } else {
      this.list = this.graphElement.ownerDocument.createElement("ol");
      this.list.className = "zeta-scm-graph-list";
      children.push(this.list);
    }
    if (graph.hasMore) children.push(this.renderMore());
    this.graphElement.replaceChildren(...children);
    this.renderRows();
    this.graphElement.setAttribute("aria-busy", this.loading ? "true" : "false");
  }

  private renderRows(): void {
    const list = this.list;
    const head = this.head;
    if (!list || !head || this.rows.length === 0) return;
    this.hovers.clear();
    const listTop = list.offsetTop;
    const viewportTop = Math.max(0, this.graphElement.scrollTop - listTop);
    const viewportHeight = Math.max(GraphRowHeight, this.graphElement.clientHeight);
    const start = Math.max(0, Math.floor(viewportTop / GraphRowHeight) - Overscan);
    const end = Math.min(this.rows.length, Math.max(start + 1, Math.ceil((viewportTop + viewportHeight) / GraphRowHeight) + Overscan));
    const children: HTMLElement[] = [this.renderSpacer(start)];
    for (let index = start; index < end; index += 1) {
      const row = this.rows[index];
      children.push(this.renderCommit(row.commit, head, this.refs.get(row.commit.objectId) ?? [], renderRow(this.graphElement.ownerDocument, row, graphNodeKind(row.commit, head))));
    }
    children.push(this.renderSpacer(this.rows.length - end));
    list.replaceChildren(...children);
  }

  private renderSpacer(rows: number): HTMLLIElement {
    const spacer = this.graphElement.ownerDocument.createElement("li");
    spacer.className = "zeta-scm-graph-spacer";
    spacer.setAttribute("aria-hidden", "true");
    spacer.style.height = `${rows * GraphRowHeight}px`;
    return spacer;
  }

  private renderMore(): HTMLDivElement {
    const container = this.graphElement.ownerDocument.createElement("div");
    container.className = "zeta-scm-graph-load-more";
    if (this.moreError) {
      const error = this.graphElement.ownerDocument.createElement("span");
      error.className = "zeta-scm-empty";
      error.textContent = this.moreError;
      container.append(error);
    }
    const button = this.graphElement.ownerDocument.createElement("button");
    button.className = "zeta-scm-command";
    button.type = "button";
    button.disabled = this.loading;
    button.textContent = this.loading ? "Loading commit history…" : this.moreError ? "Retry" : "Load more commits";
    button.setAttribute("aria-label", this.moreError ? "Retry loading commit history" : "Load more commits");
    this.more.add(addDisposableListener(button, "click", () => void this.loadMore()));
    container.append(button);
    return container;
  }

  private updateMore(): void {
    const current = this.graphElement.querySelector(".zeta-scm-graph-load-more");
    if (!this.page?.hasMore) {
      current?.remove();
      this.more.clear();
    } else if (current) {
      this.more.clear();
      current.replaceWith(this.renderMore());
    } else {
      this.graphElement.append(this.renderMore());
    }
    this.graphElement.setAttribute("aria-busy", this.loading ? "true" : "false");
  }

  private async loadMore(): Promise<void> {
    const page = this.page;
    const head = this.head;
    if (!page || !head || !page.hasMore || this.loading) return;

    const generation = this.generation;
    this.loading = true;
    this.moreError = undefined;
    this.updateMore();
    try {
      const next = await this.gitService.graph({ limit: PageSize, ...(this.cursor ? { cursor: this.cursor } : {}) });
      if (this.disposed || generation !== this.generation) return;
      const commits = [...page.commits];
      const knownObjectIds = new Set(commits.map((commit) => commit.objectId));
      const additions: GitCommitSummary[] = [];
      for (const commit of next.commits) {
        if (knownObjectIds.has(commit.objectId)) continue;
        knownObjectIds.add(commit.objectId);
        commits.push(commit);
        additions.push(commit);
      }
      const batch = createRows(additions, this.graphState);
      this.rows = [...this.rows, ...batch.rows];
      this.graphState = batch.state;
      this.page = { ...next, commits };
      this.cursor = next.nextCursor;
      this.loading = false;
      this.updateMore();
      this.renderRows();
    } catch (error) {
      if (this.disposed || generation !== this.generation) return;
      this.loading = false;
      this.moreError = gitErrorMessage(error);
      this.updateMore();
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
    this.hovers.add(this.hoverService.setupHover({
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

  private renderRemotes(graph: GraphPage): HTMLDivElement | undefined {
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

function graphNodeKind(commit: GitCommitSummary, head: GitHead): GraphNodeKind {
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
