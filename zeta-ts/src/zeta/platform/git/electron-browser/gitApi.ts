import type { GitBranchListResult, GitChangeFileResult, GitCommitChangesResult, GitCommitFileResult, GitCommitResult, GitGraphResult, GitHistoryResult, GitOperationResult, GitRepositoriesResult, GitStatusResult } from "../../../../../generated/app-server/types.js";
import { invoke } from "../../ipc/electron-browser/rendererIpc.js";
import type { IGitApi } from "../common/gitApi.js";

export function createGitApi(): IGitApi {
	return {
		repositories: () => invoke<GitRepositoriesResult>("zeta:git:repositories"),
		status: (params) => invoke<GitStatusResult>("zeta:git:status", params),
		history: (params) => invoke<GitHistoryResult>("zeta:git:history", params),
		branches: (params) => invoke<GitBranchListResult>("zeta:git:branches", params),
		switchBranch: (params) => invoke<GitOperationResult>("zeta:git:switch-branch", params),
		graph: (params) => invoke<GitGraphResult>("zeta:git:graph", params),
		commitChanges: (params) => invoke<GitCommitChangesResult>("zeta:git:commit-changes", params),
		commitFile: (params) => invoke<GitCommitFileResult>("zeta:git:commit-file", params),
		changeFile: (params) => invoke<GitChangeFileResult>("zeta:git:change-file", params),
		stage: (params) => invoke<GitOperationResult>("zeta:git:stage", params),
		unstage: (params) => invoke<GitOperationResult>("zeta:git:unstage", params),
		discardWorktree: (params) => invoke<GitOperationResult>("zeta:git:discard-worktree", params),
		commit: (params) => invoke<GitCommitResult>("zeta:git:commit", params),
		fetch: (params) => invoke<GitOperationResult>("zeta:git:fetch", params),
		pull: (params) => invoke<GitOperationResult>("zeta:git:pull", params),
		push: (params) => invoke<GitOperationResult>("zeta:git:push", params),
	};
}
