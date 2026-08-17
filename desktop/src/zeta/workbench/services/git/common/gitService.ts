import type { Event } from "../../../../base/common/event.js";
import { createServiceIdentifier } from "../../../../platform/instantiation/common/instantiation.js";

export type GitChangeStatus = "unmodified" | "modified" | "added" | "deleted" | "renamed" | "copied" | "typeChanged" | "unmerged" | "untracked" | "ignored";

export interface GitUpstream {
  readonly name: string;
  readonly ahead: number;
  readonly behind: number;
}

export type GitHead =
  | { readonly type: "branch"; readonly name: string; readonly objectId: string; readonly upstream: GitUpstream | undefined }
  | { readonly type: "detached"; readonly objectId: string }
  | { readonly type: "unborn"; readonly name: string };

export interface GitSubmoduleState {
  readonly isSubmodule: boolean;
  readonly commitChanged: boolean;
  readonly trackedChanges: boolean;
  readonly untrackedChanges: boolean;
}

export interface GitRepositoryChange {
  readonly path: string;
  readonly originalPath: string | undefined;
  readonly indexStatus: GitChangeStatus;
  readonly worktreeStatus: GitChangeStatus;
  readonly conflicted: boolean;
  readonly submodule: GitSubmoduleState;
}

export interface GitStatus {
  readonly streamInstanceId: string;
  readonly revision: number;
  readonly workspacePath: string;
  readonly head: GitHead;
  readonly changes: readonly GitRepositoryChange[];
}

export interface GitCommitSummary {
  readonly objectId: string;
  readonly parentObjectIds: readonly string[];
  readonly timestampSeconds: number;
  readonly subject: string;
}

export type GitRemoteProvider = "github" | "gitlab" | "bitbucket" | "other";

export interface GitRepositoryIdentity {
  readonly provider: GitRemoteProvider;
  readonly host: string;
  readonly owner: string;
  readonly repository: string;
}

export interface GitRemote {
  readonly name: string;
  readonly identity: GitRepositoryIdentity | undefined;
}

export type GitReferenceKind = "localBranch" | "remoteBranch";

export interface GitReference {
  readonly name: string;
  readonly objectId: string;
  readonly kind: GitReferenceKind;
  readonly remoteName: string | undefined;
  readonly current: boolean;
}

export interface GitGraph {
  readonly commits: readonly GitCommitSummary[];
  readonly references: readonly GitReference[];
  readonly remotes: readonly GitRemote[];
  readonly hasMore: boolean;
}

/** Describes one bounded page of Git graph history requested by a frontend consumer. */
export interface GitGraphQuery {
  readonly limit: number;
  readonly skip: number;
}

export interface GitCommitResult {
  readonly objectId: string;
  readonly status: GitStatus;
}

/** Frontend Git operations and repository updates for the active workspace. */
export interface IGitService {
  readonly onDidChangeStatus: Event<GitStatus>;
  readonly onDidBecomeReady: Event<void>;
  status(): Promise<GitStatus>;
  history(): Promise<readonly GitCommitSummary[]>;
  graph(query: GitGraphQuery): Promise<GitGraph>;
  stage(paths: readonly string[]): Promise<GitStatus>;
  unstage(paths: readonly string[]): Promise<GitStatus>;
  discardWorktree(paths: readonly string[]): Promise<GitStatus>;
  commit(message: string): Promise<GitCommitResult>;
  fetch(): Promise<GitStatus>;
  pull(): Promise<GitStatus>;
  push(): Promise<GitStatus>;
}

export const IGitService = createServiceIdentifier<IGitService>("gitService");
