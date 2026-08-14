import { lxiconsLibrary } from "../../../../base/common/lxiconsLibrary.js";
import { DisposableOwner, DisposableSlot } from "../../../../base/common/lifecycle.js";
import type { IWorkbenchContribution } from "../../../common/contributions.js";
import type { GitHead, GitStatus, IGitService } from "../../../services/git/common/gitService.js";
import { StatusbarAlignment, type IStatusbarEntry, type IStatusbarEntryAccessor, type IStatusbarService } from "../../../services/statusbar/browser/statusbar.js";

const BranchPriority = 900;
const PullPriority = 800;
const PushPriority = 790;

export interface ScmStatusContributionOptions {
  readonly statusbarService: IStatusbarService;
  readonly gitService: IGitService;
}

/** Projects the active Git branch and upstream state into the status bar. */
export class ScmStatusContribution extends DisposableOwner implements IWorkbenchContribution {
  private readonly branch = this.own(new DisposableSlot<IStatusbarEntryAccessor>());
  private readonly pull = this.own(new DisposableSlot<IStatusbarEntryAccessor>());
  private readonly push = this.own(new DisposableSlot<IStatusbarEntryAccessor>());
  private readonly retiredGitStreams = new Set<string>();
  private gitStatus: GitStatus | undefined;
  private refreshRevision = 0;
  private disposed = false;

  constructor(private readonly options: ScmStatusContributionOptions) {
    super();
    this.defer(() => { this.disposed = true; });
    this.own(options.gitService.onDidChangeStatus(status => {
      this.refreshRevision += 1;
      this.acceptStatus(status);
    }));
    this.own(options.gitService.onDidBecomeReady(() => this.refresh()));
    this.refresh();
  }

  private refresh(): void {
    const revision = ++this.refreshRevision;
    void this.options.gitService.status().then(status => {
      if (!this.disposed && revision === this.refreshRevision) this.acceptStatus(status);
    }).catch(() => undefined);
  }

  private acceptStatus(status: GitStatus): void {
    if (this.gitStatus) {
      if (status.streamInstanceId === this.gitStatus.streamInstanceId) {
        if (status.revision <= this.gitStatus.revision) return;
      } else {
        if (this.retiredGitStreams.has(status.streamInstanceId)) return;
        this.retiredGitStreams.add(this.gitStatus.streamInstanceId);
      }
    }
    this.gitStatus = status;
    this.updateOrAdd(this.branch, branchEntry(status.head), "zeta.status.git.branch", BranchPriority);
    const upstream = status.head.type === "branch" ? status.head.upstream : undefined;
    this.updateOrAdd(this.pull, transferEntry("pull", upstream?.behind ?? 0, upstream?.name), "zeta.status.git.pull", PullPriority);
    this.updateOrAdd(this.push, transferEntry("push", upstream?.ahead ?? 0, upstream?.name), "zeta.status.git.push", PushPriority);
  }

  private updateOrAdd(slot: DisposableSlot<IStatusbarEntryAccessor>, entry: IStatusbarEntry, id: string, priority: number): void {
    if (slot.value) {
      slot.value.update(entry);
      return;
    }
    slot.replace(this.options.statusbarService.addEntry(entry, { id, alignment: StatusbarAlignment.Left, priority }));
  }
}

function branchEntry(head: GitHead): IStatusbarEntry {
  switch (head.type) {
    case "branch": return { icon: lxiconsLibrary.gitBranch, text: head.name, ariaLabel: `Git branch ${head.name}`, tooltip: head.upstream ? `${head.name} tracks ${head.upstream.name}` : `${head.name} has no upstream` };
    case "unborn": return { icon: lxiconsLibrary.gitBranch, text: head.name, ariaLabel: `Unborn Git branch ${head.name}`, tooltip: `${head.name} has no commits` };
    case "detached": {
      const revision = head.objectId.slice(0, 8);
      return { icon: lxiconsLibrary.gitCommit, text: revision, ariaLabel: `Detached Git HEAD at ${revision}`, tooltip: `Detached HEAD at ${head.objectId}` };
    }
  }
}

function transferEntry(direction: "pull" | "push", count: number, upstream: string | undefined): IStatusbarEntry {
  const action = direction === "pull" ? "incoming" : "outgoing";
  return {
    icon: direction === "pull" ? lxiconsLibrary.repoPull : lxiconsLibrary.repoPush,
    text: String(count),
    ariaLabel: `${count} ${action} Git ${count === 1 ? "change" : "changes"}`,
    tooltip: upstream ? `${count} ${action} from ${upstream}` : "No upstream branch configured",
  };
}
