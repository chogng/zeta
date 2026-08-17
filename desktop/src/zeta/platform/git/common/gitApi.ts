import type { GitCommitParams, GitCommitResult, GitGraphResult, GitHistoryResult, GitOperationResult, GitPathsParams, GitStatusResult } from "../../../../../generated/app-server/types.js";

export interface IGitApi {
  status(): Promise<GitStatusResult>;
  history(): Promise<GitHistoryResult>;
  graph(): Promise<GitGraphResult>;
  stage(params: GitPathsParams): Promise<GitOperationResult>;
  unstage(params: GitPathsParams): Promise<GitOperationResult>;
  discardWorktree(params: GitPathsParams): Promise<GitOperationResult>;
  commit(params: GitCommitParams): Promise<GitCommitResult>;
  fetch(): Promise<GitOperationResult>;
  pull(): Promise<GitOperationResult>;
  push(): Promise<GitOperationResult>;
}
