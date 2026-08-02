import { addDisposableListener } from "../../../../base/browser/dom.js";
import { Button } from "../../../../base/browser/ui/button/button.js";
import { lxiconsLibrary } from "../../../../base/common/lxiconsLibrary.js";
import type { GitChangeStatus, GitHead, GitRepositoryChange, GitStatus, IGitService } from "../../../services/git/common/gitService.js";
import { ViewPane, type IViewPaneOptions } from "../../../browser/parts/views/viewPane.js";

type GitChangeSide = "index" | "worktree";
type GitPathAction = "stage" | "unstage" | "discard";

/** Git status and user mutations routed through the workspace App Server. */
export class ScmViewPane extends ViewPane {
  private readonly gitService: IGitService;
  private readonly branchElement: HTMLDivElement;
  private readonly commitInput: HTMLTextAreaElement;
  private readonly commitButton: HTMLButtonElement;
  private readonly statusElement: HTMLDivElement;
  private readonly changesElement: HTMLDivElement;
  private status: GitStatus | undefined;
  private readonly retiredStreamInstanceIds = new Set<string>();
  private revision = 0;
  private busy = false;
  private disposed = false;

  constructor(options: IViewPaneOptions, gitService: IGitService) {
    super(options);
    this.gitService = gitService;
    this.contentElement.classList.add("zeta-scm");
    const document = options.ownerDocument;
    const summary = document.createElement("div");
    summary.className = "zeta-scm-summary";
    this.branchElement = document.createElement("div");
    this.branchElement.className = "zeta-scm-branch";
    this.branchElement.textContent = "Loading repository…";
    summary.append(this.branchElement);
    const commitForm = document.createElement("form");
    commitForm.className = "zeta-scm-commit-form";
    this.commitInput = document.createElement("textarea");
    this.commitInput.className = "zeta-scm-commit-input";
    this.commitInput.name = "commitMessage";
    this.commitInput.rows = 2;
    this.commitInput.placeholder = "Message (Ctrl+Enter to commit)";
    this.commitInput.setAttribute("aria-label", "Commit message");
    const commitButton = this.own(new Button({
      label: "Commit",
      icon: lxiconsLibrary.check,
      contentAlignment: "labelCentered",
      ownerDocument: document,
      title: "Commit staged changes",
    }));
    this.commitButton = commitButton.element;
    this.commitButton.classList.add("zeta-scm-commit");
    this.commitButton.type = "submit";
    commitForm.append(this.commitInput, this.commitButton);
    this.statusElement = document.createElement("div");
    this.statusElement.className = "zeta-scm-status";
    this.statusElement.setAttribute("role", "status");
    this.statusElement.setAttribute("aria-live", "polite");
    this.statusElement.textContent = "Reading Git status…";
    this.changesElement = document.createElement("div");
    this.changesElement.className = "zeta-scm-changes";
    this.contentElement.append(summary, commitForm, this.statusElement, this.changesElement);
    this.own(addDisposableListener(commitForm, "submit", (event) => {
      event.preventDefault();
      void this.commit();
    }));
    this.own(addDisposableListener(this.commitInput, "keydown", (event) => {
      const keyboardEvent = event as KeyboardEvent;
      if (keyboardEvent.key === "Enter" && (keyboardEvent.ctrlKey || keyboardEvent.metaKey)) {
        event.preventDefault();
        void this.commit();
      }
    }));
    this.own(addDisposableListener(this.changesElement, "click", (event) => this.onChangeAction(event)));
    this.own(this.gitService.onDidChangeStatus((status) => this.onStatusChanged(status)));
    this.own(this.gitService.onDidBecomeReady(() => void this.refresh()));
    this.defer(() => {
      this.disposed = true;
      this.revision += 1;
    });
    this.setBusy(true);
    void this.refresh();
  }

  async refresh(): Promise<void> {
    const revision = ++this.revision;
    this.setBusy(true);
    this.statusElement.textContent = "Reading Git status…";
    try {
      const status = await this.gitService.status();
      if (this.disposed || revision !== this.revision) return;
      this.renderStatus(status);
    } catch (error) {
      this.renderError(error, revision);
    } finally {
      if (!this.disposed && revision === this.revision) this.setBusy(false);
    }
  }

  private async commit(): Promise<void> {
    const message = this.commitInput.value.trim();
    if (!message) {
      this.statusElement.textContent = "Enter a commit message.";
      this.commitInput.focus();
      return;
    }
    const revision = ++this.revision;
    this.setBusy(true);
    this.statusElement.textContent = "Committing staged changes…";
    try {
      const result = await this.gitService.commit(message);
      if (this.disposed || revision !== this.revision) return;
      this.commitInput.value = "";
      this.renderStatus(result.status, `Created commit ${result.objectId.slice(0, 7)}.`);
    } catch (error) {
      this.renderError(error, revision);
    } finally {
      if (!this.disposed && revision === this.revision) this.setBusy(false);
    }
  }

  private onChangeAction(event: Event): void {
    const target = event.target;
    const HTMLElementConstructor = this.element.ownerDocument.defaultView?.HTMLElement;
    if (!HTMLElementConstructor || !(target instanceof HTMLElementConstructor) || this.busy) return;
    const button = target.closest<HTMLButtonElement>("button[data-scm-action]");
    if (!button || !this.changesElement.contains(button)) return;
    const action = button.dataset.scmAction as GitPathAction | "stageAll" | "unstageAll";
    const paths = button.dataset.scmPaths
      ? parseActionPaths(button.dataset.scmPaths)
      : button.dataset.scmPath
      ? [button.dataset.scmPath]
      : [];
    if (paths.length === 0) return;
    if (action === "discard") {
      const confirmed = this.element.ownerDocument.defaultView?.confirm(
        `Discard working-tree changes in ${paths[0]}? This cannot be undone.`,
      ) === true;
      if (!confirmed) return;
    }
    void this.runPathAction(action === "stageAll" ? "stage" : action === "unstageAll" ? "unstage" : action, paths);
  }

  private async runPathAction(action: GitPathAction, paths: readonly string[]): Promise<void> {
    const revision = ++this.revision;
    this.setBusy(true);
    this.statusElement.textContent = `${pathActionLabel(action)} ${paths.length === 1 ? paths[0] : `${paths.length} paths`}…`;
    try {
      const result = action === "stage"
        ? await this.gitService.stage(paths)
        : action === "unstage"
        ? await this.gitService.unstage(paths)
        : await this.gitService.discardWorktree(paths);
      if (this.disposed || revision !== this.revision) return;
      this.renderStatus(result);
    } catch (error) {
      this.renderError(error, revision);
    } finally {
      if (!this.disposed && revision === this.revision) this.setBusy(false);
    }
  }

  private onStatusChanged(status: GitStatus): void {
    if (this.disposed) return;
    if (
      this.status &&
      status.streamInstanceId === this.status.streamInstanceId &&
      status.revision <= this.status.revision
    ) return;
    this.renderStatus(status);
  }

  private renderStatus(status: GitStatus, announcement?: string): void {
    if (this.status) {
      if (status.streamInstanceId === this.status.streamInstanceId) {
        if (status.revision < this.status.revision) return;
      } else {
        if (this.retiredStreamInstanceIds.has(status.streamInstanceId)) return;
        this.retiredStreamInstanceIds.add(this.status.streamInstanceId);
      }
    }
    this.status = status;
    this.branchElement.textContent = headLabel(status.head);
    this.branchElement.title = headTitle(status.head);
    this.changesElement.replaceChildren();
    const conflicts = status.changes.filter((change) => change.conflicted);
    const staged = status.changes.filter((change) => !change.conflicted && change.indexStatus !== "unmodified");
    const working = status.changes.filter((change) => !change.conflicted && change.worktreeStatus !== "unmodified");
    const summary = conflicts.length === 0 && staged.length === 0 && working.length === 0
      ? "No changes."
      : `${status.changes.length} changed ${status.changes.length === 1 ? "file" : "files"}`;
    this.statusElement.textContent = announcement ? `${announcement} ${summary}` : summary;
    this.appendSection("Merge Changes", conflicts, "worktree", "stageAll");
    this.appendSection("Staged Changes", staged, "index", "unstageAll");
    this.appendSection("Changes", working, "worktree", "stageAll");
    this.updateCommandState();
  }

  private appendSection(title: string, changes: readonly GitRepositoryChange[], side: GitChangeSide, action: "stageAll" | "unstageAll"): void {
    if (changes.length === 0) return;
    const document = this.element.ownerDocument;
    const section = document.createElement("section");
    section.className = "zeta-scm-section";
    const heading = document.createElement("h3");
    heading.className = "zeta-scm-section-heading";
    const label = document.createElement("span");
    label.textContent = title;
    const count = document.createElement("span");
    count.className = "zeta-scm-section-count";
    count.textContent = String(changes.length);
    const all = commandButton(document, action === "stageAll" ? "Stage All" : "Unstage All", `${action === "stageAll" ? "Stage" : "Unstage"} all ${title.toLowerCase()}`);
    all.classList.add("zeta-scm-section-action");
    all.dataset.scmAction = action;
    all.dataset.scmPaths = JSON.stringify([...new Set(changes.flatMap(changePaths))]);
    heading.append(label, count, all);
    const list = document.createElement("ul");
    list.className = "zeta-scm-list";
    for (const change of changes) list.append(renderChange(document, change, side));
    section.append(heading, list);
    this.changesElement.append(section);
  }

  private renderError(error: unknown, revision: number): void {
    if (this.disposed || revision !== this.revision) return;
    this.statusElement.textContent = gitErrorMessage(error);
  }

  private setBusy(busy: boolean): void {
    this.busy = busy;
    this.updateCommandState();
  }

  private updateCommandState(): void {
    const hasStagedChanges = (this.status?.changes ?? []).some((change) => !change.conflicted && change.indexStatus !== "unmodified");
    this.commitButton.disabled = this.busy || !hasStagedChanges;
    this.commitInput.disabled = this.busy;
    for (const button of this.changesElement.querySelectorAll<HTMLButtonElement>(".zeta-scm-command")) {
      button.disabled = this.busy;
    }
  }
}

function commandButton(document: Document, text: string, ariaLabel: string): HTMLButtonElement {
  const button = document.createElement("button");
  button.type = "button";
  button.className = "zeta-scm-command";
  button.textContent = text;
  button.setAttribute("aria-label", ariaLabel);
  return button;
}

function renderChange(document: Document, change: GitRepositoryChange, side: GitChangeSide): HTMLLIElement {
  const item = document.createElement("li");
  item.className = "zeta-scm-change";
  const path = document.createElement("span");
  path.className = "zeta-scm-change-path";
  path.textContent = change.originalPath ? `${change.originalPath} → ${change.path}` : change.path;
  path.title = path.textContent;
  const actions = document.createElement("span");
  actions.className = "zeta-scm-change-actions";
  if (side === "index") {
    actions.append(changeAction(document, "unstage", changePaths(change), "Unstage"));
  } else {
    actions.append(changeAction(document, "stage", changePaths(change), "Stage"));
    if (!change.conflicted && ["modified", "deleted", "typeChanged"].includes(change.worktreeStatus)) {
      actions.append(changeAction(document, "discard", [change.path], "Discard"));
    }
  }
  const status = side === "index" ? change.indexStatus : change.worktreeStatus;
  const badge = document.createElement("span");
  badge.className = `zeta-scm-change-status status-${status}`;
  badge.textContent = statusCode(status);
  badge.title = statusLabel(status);
  item.append(path, actions, badge);
  return item;
}

function changeAction(document: Document, action: GitPathAction, paths: readonly string[], label: string): HTMLButtonElement {
  const button = commandButton(document, label, `${label} ${paths[0]}`);
  button.classList.add("zeta-scm-change-action");
  button.dataset.scmAction = action;
  button.dataset.scmPaths = JSON.stringify(paths);
  return button;
}

function changePaths(change: GitRepositoryChange): string[] {
  return change.originalPath ? [change.originalPath, change.path] : [change.path];
}

function parseActionPaths(value: string): string[] {
  try {
    const paths: unknown = JSON.parse(value);
    return Array.isArray(paths) && paths.every((path) => typeof path === "string") ? paths : [];
  } catch {
    return [];
  }
}

function headLabel(head: GitHead): string {
  switch (head.type) {
    case "branch": return `${head.name}${upstreamDistance(head.upstream)}`;
    case "detached": return `Detached at ${head.objectId.slice(0, 7)}`;
    case "unborn": return head.name;
  }
}

function headTitle(head: GitHead): string {
  switch (head.type) {
    case "branch": return head.upstream ? `${head.name} tracks ${head.upstream.name}` : head.name;
    case "detached": return `Detached HEAD ${head.objectId}`;
    case "unborn": return `${head.name} has no commits`;
  }
}

function upstreamDistance(upstream: Extract<GitHead, { type: "branch" }>["upstream"]): string {
  if (!upstream) return "";
  const parts = [];
  if (upstream.ahead > 0) parts.push(`↑${upstream.ahead}`);
  if (upstream.behind > 0) parts.push(`↓${upstream.behind}`);
  return parts.length > 0 ? ` ${parts.join(" ")}` : "";
}

function statusCode(status: GitChangeStatus): string {
  switch (status) {
    case "modified": return "M";
    case "added": return "A";
    case "deleted": return "D";
    case "renamed": return "R";
    case "copied": return "C";
    case "typeChanged": return "T";
    case "unmerged": return "!";
    case "untracked": return "U";
    case "ignored": return "I";
    case "unmodified": return "";
  }
}

function statusLabel(status: GitChangeStatus): string {
  return status.replace(/([A-Z])/g, " $1").toLowerCase();
}

function pathActionLabel(action: GitPathAction): string {
  switch (action) {
    case "stage": return "Staging";
    case "unstage": return "Unstaging";
    case "discard": return "Discarding";
  }
}

function gitErrorMessage(error: unknown): string {
  if (error instanceof Error && /GitNotRepository/.test(error.message)) {
    return "The open folder is not a Git repository.";
  }
  return error instanceof Error ? error.message : "Git operation failed.";
}
