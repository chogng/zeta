import type { GitChangeFileParams, GitChangeFileResult, GitCommitChangesParams, GitCommitChangesResult, GitCommitFileParams, GitCommitFileResult, GitCommitParams, GitCommitResult, GitGraphParams, GitGraphResult, GitHistoryResult, GitOperationResult, GitPathsParams, GitStatusResult } from "../../../../../generated/app-server/types.js";

export interface IGitApi {
  status(): Promise<GitStatusResult>;
  history(): Promise<GitHistoryResult>;
  graph(params: GitGraphParams): Promise<GitGraphResult>;
  commitChanges(params: GitCommitChangesParams): Promise<GitCommitChangesResult>;
  commitFile(params: GitCommitFileParams): Promise<GitCommitFileResult>;
  changeFile(params: GitChangeFileParams): Promise<GitChangeFileResult>;
  stage(params: GitPathsParams): Promise<GitOperationResult>;
  unstage(params: GitPathsParams): Promise<GitOperationResult>;
  discardWorktree(params: GitPathsParams): Promise<GitOperationResult>;
  commit(params: GitCommitParams): Promise<GitCommitResult>;
  fetch(): Promise<GitOperationResult>;
  pull(): Promise<GitOperationResult>;
  push(): Promise<GitOperationResult>;
}
