import type { GitHeadDto, GitRepositoryChangeDto, GitStatusResult } from "../../../../../../generated/app-server/types.js";
import { Emitter } from "../../../../base/common/event.js";
import { DisposableOwner } from "../../../../base/common/lifecycle.js";
import type { IAppServerApi, IServerEventApi } from "../../../../platform/app-server/common/appServerApi.js";
import type { IGitApi } from "../../../../platform/git/common/gitApi.js";
import type { GitCommitResult, GitCommitSummary, GraphPage, GraphQuery, GitHead, GitRepositoryChange, GitStatus, IGitService } from "../common/gitService.js";

export interface GitServiceOptions {
  readonly api: IGitApi;
  readonly appServerApi: IAppServerApi;
  readonly eventApi: IServerEventApi;
}

/** App Server-backed implementation of the frontend Git service. */
export class GitService extends DisposableOwner implements IGitService {
  private readonly _onDidChangeStatus = this.own(new Emitter<GitStatus>());
  private readonly _onDidBecomeReady = this.own(new Emitter<void>());
  private readonly api: IGitApi;

  readonly onDidChangeStatus = this._onDidChangeStatus.event;
  readonly onDidBecomeReady = this._onDidBecomeReady.event;

  constructor(options: GitServiceOptions) {
    super();
    this.api = options.api;
    const events = options.eventApi.subscribe((event) => {
      if (event.method === "git/statusChanged") this._onDidChangeStatus.fire(toGitStatus(event.params.status));
    });
    this.defer(() => events.dispose());
    const connection = options.appServerApi.onConnectionState((state) => {
      if (state === "ready") this._onDidBecomeReady.fire();
    });
    this.defer(() => connection.dispose());
  }

  async status(): Promise<GitStatus> {
    return toGitStatus(await this.api.status());
  }

  async history(): Promise<readonly GitCommitSummary[]> {
    const result = await this.api.history();
    return result.commits.map((commit) => ({ ...commit }));
  }

  async graph(query: GraphQuery): Promise<GraphPage> {
    const result = await this.api.graph({ limit: query.limit, ...(query.cursor ? { cursor: query.cursor } : {}) });
    return {
      commits: result.commits.map((commit) => ({ ...commit, parentObjectIds: [...commit.parentObjectIds] })),
      references: result.references.map((reference) => ({ ...reference, remoteName: reference.remoteName ?? undefined })),
      remotes: result.remotes.map((remote) => ({
        name: remote.name,
        identity: remote.identity ? { ...remote.identity } : undefined,
      })),
      hasMore: result.hasMore,
      nextCursor: result.nextCursor,
    };
  }

  async stage(paths: readonly string[]): Promise<GitStatus> {
    return toGitStatus((await this.api.stage({ paths: [...paths] })).status);
  }

  async unstage(paths: readonly string[]): Promise<GitStatus> {
    return toGitStatus((await this.api.unstage({ paths: [...paths] })).status);
  }

  async discardWorktree(paths: readonly string[]): Promise<GitStatus> {
    return toGitStatus((await this.api.discardWorktree({ paths: [...paths] })).status);
  }

  async commit(message: string): Promise<GitCommitResult> {
    const result = await this.api.commit({ message });
    return { objectId: result.objectId, status: toGitStatus(result.status) };
  }

  async fetch(): Promise<GitStatus> {
    return toGitStatus((await this.api.fetch()).status);
  }

  async pull(): Promise<GitStatus> {
    return toGitStatus((await this.api.pull()).status);
  }

  async push(): Promise<GitStatus> {
    return toGitStatus((await this.api.push()).status);
  }
}

function toGitStatus(status: GitStatusResult): GitStatus {
  return {
    streamInstanceId: status.streamInstanceId,
    revision: status.revision,
    workspacePath: status.workspacePath,
    head: toGitHead(status.head),
    changes: status.changes.map(toGitChange),
  };
}

function toGitHead(head: GitHeadDto): GitHead {
  switch (head.type) {
    case "branch": return { type: "branch", name: head.name, objectId: head.objectId, upstream: head.upstream ? { ...head.upstream } : undefined };
    case "detached": return { type: "detached", objectId: head.objectId };
    case "unborn": return { type: "unborn", name: head.name };
  }
}

function toGitChange(change: GitRepositoryChangeDto): GitRepositoryChange {
  return {
    path: change.path,
    originalPath: change.originalPath ?? undefined,
    indexStatus: change.indexStatus,
    worktreeStatus: change.worktreeStatus,
    conflicted: change.conflicted,
    submodule: { ...change.submodule },
  };
}
