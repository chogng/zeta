import type { GitCommitResult, GitHistoryResult, GitOperationResult, GitStatusResult } from "../../../../../generated/app-server/types.js";
import { invoke } from "../../ipc/electron-browser/rendererIpc.js";
import type { IGitApi } from "../common/gitApi.js";

export function createGitApi(): IGitApi {
  return {
    status: () => invoke<GitStatusResult>("zeta:git:status"),
    history: () => invoke<GitHistoryResult>("zeta:git:history"),
    stage: (params) => invoke<GitOperationResult>("zeta:git:stage", params),
    unstage: (params) => invoke<GitOperationResult>("zeta:git:unstage", params),
    discardWorktree: (params) => invoke<GitOperationResult>("zeta:git:discard-worktree", params),
    commit: (params) => invoke<GitCommitResult>("zeta:git:commit", params),
    fetch: () => invoke<GitOperationResult>("zeta:git:fetch"),
    pull: () => invoke<GitOperationResult>("zeta:git:pull"),
    push: () => invoke<GitOperationResult>("zeta:git:push"),
  };
}
