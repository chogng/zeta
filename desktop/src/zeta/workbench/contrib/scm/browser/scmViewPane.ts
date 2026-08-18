import { addDisposableListener, h } from "../../../../base/browser/dom.js";
import { Button } from "../../../../base/browser/ui/button/button.js";
import { IconLabel } from "../../../../base/browser/ui/iconlabel/iconlabel.js";
import { ToolBar } from "../../../../base/browser/ui/toolbar/toolbar.js";
import type { IAction } from "../../../../base/common/actions.js";
import { lxiconsLibrary } from "../../../../base/common/lxiconsLibrary.js";
import { ResettableDisposableGroup } from "../../../../base/common/lifecycle.js";
import { URI } from "../../../../base/common/uri.js";
import type { IFileIconThemeService } from "../../../../platform/theme/browser/fileIconThemeService.js";
import type { GitChangeFileComparison, GitChangeStatus, GitCommitFileContent, GitRepositoryChange, GitStatus, IGitService } from "../../../services/git/common/gitService.js";
import type { IEditorService } from "../../../services/editor/common/editorService.js";
import { ViewPane, type IViewPaneOptions } from "../../../browser/parts/views/viewPane.js";
import { createDiffEditorInput } from "../../codeEditor/browser/diffEditorInput.js";
import { gitErrorMessage } from "./scmError.js";

type GitChangeSide = "index" | "worktree";
type GitPathAction = "stage" | "unstage" | "discard";

const inactiveContextMenuProvider = { showContextMenu(): void {} };

/** Git status and user mutations routed through the workspace App Server. */
export class ScmViewPane extends ViewPane {
  private readonly gitService: IGitService;
  private readonly commitInput: HTMLTextAreaElement;
  private readonly commitButton: HTMLButtonElement;
  private readonly statusElement: HTMLDivElement;
  private readonly changesElement: HTMLDivElement;
  private readonly renderedChanges = this.own(new ResettableDisposableGroup());
  private status: GitStatus | undefined;
  private readonly retiredStreamInstanceIds = new Set<string>();
  private revision = 0;
  private busy = false;
  private unavailable = false;
  private disposed = false;

  constructor(options: IViewPaneOptions, gitService: IGitService, private readonly fileIconThemeService: IFileIconThemeService, private readonly editorService: IEditorService) {
    super(options);
    this.gitService = gitService;
    this.contentElement.classList.add("zeta-scm");
    const document = options.ownerDocument;
    const commitForm = h(document, "form");
    commitForm.className = "zeta-scm-commit-form";
    this.commitInput = h(document, "textarea");
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
    this.statusElement = h(document, "div");
    this.statusElement.className = "zeta-scm-status zeta-aria-live";
    this.statusElement.setAttribute("role", "status");
    this.statusElement.setAttribute("aria-live", "polite");
    this.statusElement.textContent = "Reading Git status…";
    this.changesElement = h(document, "div");
    this.changesElement.className = "zeta-scm-changes";
    this.contentElement.append(commitForm, this.statusElement, this.changesElement);
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
    this.own(this.gitService.onDidChangeStatus((status) => this.onStatusChanged(status)));
    this.own(this.gitService.onDidBecomeReady(() => void this.refresh()));
    this.own(this.fileIconThemeService.onDidFileIconThemeChange(() => {
      if (this.status) this.renderStatus(this.status);
    }));
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

  private requestPathAction(action: GitPathAction, paths: readonly string[]): void {
    if (this.busy || paths.length === 0) return;
    if (action === "discard") {
      const target = paths.length === 1 ? paths[0] : `${paths.length} working-tree files`;
      const confirmed = this.element.ownerDocument.defaultView?.confirm(
        `Discard changes in ${target}? This cannot be undone.`,
      ) === true;
      if (!confirmed) return;
    }
    void this.runPathAction(action, paths);
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
    this.unavailable = false;
    this.renderedChanges.clear();
    this.changesElement.replaceChildren();
    const conflicts = status.changes.filter((change) => change.conflicted);
    const staged = status.changes.filter((change) => !change.conflicted && change.indexStatus !== "unmodified");
    const working = status.changes.filter((change) => !change.conflicted && change.worktreeStatus !== "unmodified");
    const summary = conflicts.length === 0 && staged.length === 0 && working.length === 0
      ? "No changes."
      : `${status.changes.length} changed ${status.changes.length === 1 ? "file" : "files"}`;
    this.statusElement.textContent = announcement ? `${announcement} ${summary}` : summary;
    this.appendSection("Merge Changes", conflicts, "worktree");
    this.appendSection("Staged Changes", staged, "index");
    this.appendSection("Changes", working, "worktree");
    this.updateCommandState();
  }

  private appendSection(title: string, changes: readonly GitRepositoryChange[], side: GitChangeSide): void {
    if (changes.length === 0) return;
    const document = this.element.ownerDocument;
    const section = h(document, "section");
    section.className = "zeta-scm-section";
    const heading = h(document, "h3");
    heading.className = "zeta-scm-section-heading";
    heading.tabIndex = 0;
    const label = h(document, "span");
    label.className = "zeta-scm-section-label";
    label.textContent = title;
    const actions = this.renderActionToolbar(
      sectionActions(title, changes, side).map((action) => this.pathAction(action.id, action.label, action.action, action.paths)),
      `${title} actions`,
    );
    actions.classList.add("zeta-scm-section-actions");
    const count = h(document, "span");
    count.className = "zeta-scm-section-count";
    count.textContent = String(changes.length);
    heading.append(label, actions, count);
    const list = h(document, "ul");
    list.className = "zeta-scm-list";
    for (const change of changes) list.append(this.renderChange(change, side));
    section.append(heading, list);
    this.changesElement.append(section);
  }

  private renderChange(change: GitRepositoryChange, side: GitChangeSide): HTMLLIElement {
    const document = this.element.ownerDocument;
    const item = h(document, "li");
    item.className = "zeta-scm-change";
    const name = basename(change.path);
    const parentPath = dirname(change.path);
    const fileLabel = this.renderedChanges.add(new IconLabel({
      label: name,
      ownerDocument: document,
      reserveIconSpace: true,
      renderIcon: (container) => this.fileIconThemeService.renderFileIcon(repositoryFileUri(this.status?.workspacePath, change.path), container),
      title: change.originalPath ? `${change.originalPath} → ${change.path}` : change.path,
    }));
    fileLabel.element.classList.add("zeta-scm-change-label");
    if (parentPath) {
      const description = h(document, "span");
      description.className = "zeta-scm-change-description";
      description.textContent = parentPath;
      fileLabel.element.append(description);
    }
    const open = h(document, "button");
    open.type = "button";
    open.className = "zeta-scm-change-open";
    open.setAttribute("aria-label", `Open ${side === "index" ? "staged changes" : "changes"} for ${change.path}`);
    open.append(fileLabel.element);
    if (change.conflicted) {
      open.disabled = true;
      open.setAttribute("aria-label", `Merge conflict in ${change.path}`);
    } else {
      this.renderedChanges.add(addDisposableListener(open, "click", (event) => {
        if ((event as MouseEvent).detail > 1) return;
        void this.openChange(change, side, false);
      }));
      this.renderedChanges.add(addDisposableListener(open, "dblclick", () => {
        void this.openChange(change, side, true);
      }));
    }
    const actions = side === "index"
      ? [this.pathAction(`scm.change.unstage.${change.path}`, `Unstage ${change.path}`, "unstage", changePaths(change))]
      : [
        ...(isDiscardable(change) ? [this.pathAction(`scm.change.discard.${change.path}`, `Discard ${change.path}`, "discard", [change.path])] : []),
        this.pathAction(`scm.change.stage.${change.path}`, `Stage ${change.path}`, "stage", changePaths(change)),
      ];
    const toolbar = this.renderActionToolbar(actions, `Actions for ${change.path}`);
    toolbar.classList.add("zeta-scm-change-actions");
    const status = side === "index" ? change.indexStatus : change.worktreeStatus;
    const badge = h(document, "span");
    badge.className = `zeta-scm-change-status status-${status}`;
    badge.textContent = statusCode(status);
    badge.title = statusLabel(status);
    item.append(open, toolbar, badge);
    return item;
  }

  private async openChange(change: GitRepositoryChange, side: GitChangeSide, pinned: boolean): Promise<void> {
    const status = this.status;
    if (!status) return;
    const comparison: GitChangeFileComparison = side === "index" ? "staged" : "unstaged";
    try {
      const file = await this.gitService.changeFile(change.path, comparison);
      if (this.disposed) return;
      const originalPath = changeOriginalPath(change, comparison);
      const [originalState, modifiedState] = comparison === "staged"
        ? ["HEAD", "Index"] as const
        : ["Index", "Working Tree"] as const;
      const original = changeEditorInput(file.original, changeFileUri(status, comparison, originalPath, "original"), `${basename(originalPath)} (${originalState})`);
      const modified = changeEditorInput(file.modified, changeFileUri(status, comparison, change.path, "modified"), `${basename(change.path)} (${modifiedState})`);
      if (original && modified) {
        await this.editorService.openEditor(createDiffEditorInput(original, modified, `${original.label} ↔ ${modified.label}`), { pinned });
      } else if (modified) {
        await this.editorService.openEditor(modified, { pinned });
      } else if (original) {
        await this.editorService.openEditor(original, { pinned });
      }
    } catch (error) {
      if (!this.disposed) this.statusElement.textContent = gitErrorMessage(error);
    }
  }

  private pathAction(id: string, label: string, action: GitPathAction, paths: readonly string[]): IAction {
    return {
      id,
      label,
      tooltip: label,
      icon: action === "stage" ? lxiconsLibrary.add : action === "unstage" ? lxiconsLibrary.remove : lxiconsLibrary.discard,
      enabled: true,
      checked: undefined,
      run: () => this.requestPathAction(action, paths),
    };
  }

  private renderActionToolbar(actions: readonly IAction[], ariaLabel: string): HTMLDivElement {
    const toolbar = this.renderedChanges.add(new ToolBar({
      ownerDocument: this.element.ownerDocument,
      contextMenuProvider: inactiveContextMenuProvider,
      ariaLabel,
    }));
    toolbar.setActions(actions);
    toolbar.element.classList.add("zeta-scm-action-toolbar");
    for (const button of toolbar.element.querySelectorAll<HTMLButtonElement>("button")) {
      button.classList.add("zeta-scm-action-button");
      const item = button.closest<HTMLElement>(".zeta-action-view-item");
      const action = actions.find((candidate) => candidate.id === item?.dataset.actionId);
      if (action) {
        button.setAttribute("aria-label", action.label);
        if (action.icon) button.dataset.icon = action.icon.id;
      }
    }
    return toolbar.element;
  }

  private renderError(error: unknown, revision: number): void {
    if (this.disposed || revision !== this.revision) return;
    this.status = undefined;
    this.unavailable = true;
    this.renderedChanges.clear();
    this.changesElement.replaceChildren();
    this.statusElement.textContent = gitErrorMessage(error);
    this.updateCommandState();
  }

  private setBusy(busy: boolean): void {
    this.busy = busy;
    this.updateCommandState();
  }

  private updateCommandState(): void {
    const hasStagedChanges = (this.status?.changes ?? []).some((change) => !change.conflicted && change.indexStatus !== "unmodified");
    this.commitButton.disabled = this.busy || !hasStagedChanges;
    this.commitInput.disabled = this.busy || this.unavailable;
    for (const button of this.changesElement.querySelectorAll<HTMLButtonElement>(".zeta-scm-action-button")) {
      button.disabled = this.busy;
    }
  }
}

function changePaths(change: GitRepositoryChange): string[] {
  return change.originalPath ? [change.originalPath, change.path] : [change.path];
}

function isDiscardable(change: GitRepositoryChange): boolean {
  return !change.conflicted && ["modified", "deleted", "typeChanged"].includes(change.worktreeStatus);
}

function sectionActions(title: string, changes: readonly GitRepositoryChange[], side: GitChangeSide): readonly { readonly id: string; readonly label: string; readonly action: GitPathAction; readonly paths: readonly string[] }[] {
  if (side === "index") {
    return [{ id: "scm.section.unstageAll", label: "Unstage All Changes", action: "unstage", paths: uniquePaths(changes.flatMap(changePaths)) }];
  }
  const actions = [];
  const discardable = changes.filter(isDiscardable).map((change) => change.path);
  if (discardable.length > 0) actions.push({ id: "scm.section.discardAll", label: "Discard All Changes", action: "discard" as const, paths: uniquePaths(discardable) });
  actions.push({ id: `scm.section.stageAll.${title === "Merge Changes" ? "merge" : "changes"}`, label: title === "Merge Changes" ? "Stage All Merge Changes" : "Stage All Changes", action: "stage" as const, paths: uniquePaths(changes.flatMap(changePaths)) });
  return actions;
}

function uniquePaths(paths: readonly string[]): string[] {
  return [...new Set(paths)];
}

function basename(path: string): string {
  return path.replaceAll("\\", "/").split("/").at(-1) ?? path;
}

function dirname(path: string): string {
  const normalized = path.replaceAll("\\", "/");
  const separator = normalized.lastIndexOf("/");
  return separator < 0 ? "" : normalized.slice(0, separator);
}

function repositoryFileUri(workspacePath: string | undefined, path: string): URI {
  const normalizedPath = path.replaceAll("\\", "/").replace(/^\/+/, "");
  const normalizedWorkspace = workspacePath?.replaceAll("\\", "/").replace(/\/+$/, "");
  if (normalizedWorkspace && (normalizedWorkspace.startsWith("/") || /^[A-Za-z]:\//.test(normalizedWorkspace))) {
    return URI.file(`${normalizedWorkspace}/${normalizedPath}`);
  }
  return URI.parse(`file:///${normalizedPath.split("/").map(encodeURIComponent).join("/")}`);
}

function changeOriginalPath(change: GitRepositoryChange, comparison: GitChangeFileComparison): string {
  const status = comparison === "staged" ? change.indexStatus : change.worktreeStatus;
  return status === "renamed" || status === "copied" ? change.originalPath ?? change.path : change.path;
}

function changeEditorInput(content: GitCommitFileContent, resource: URI, label: string) {
  if (content.kind === "binary") return undefined;
  return {
    resource,
    label,
    readOnly: true,
    initialText: content.kind === "missing" ? "" : content.text,
  };
}

function changeFileUri(status: GitStatus, comparison: GitChangeFileComparison, path: string, side: "original" | "modified"): URI {
  const encodedPath = path.split("/").map(encodeURIComponent).join("/");
  const query = new URLSearchParams({
    side,
    stream: status.streamInstanceId,
    revision: String(status.revision),
  });
  return URI.parse(`git-change:/${comparison}/${encodedPath}?${query}`);
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
