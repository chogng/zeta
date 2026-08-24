import type { ViteDevAppServerConnection } from "../../app-server/browser/viteDevConnection.js";
import { viteDevRequest } from "../../app-server/browser/viteDevRequest.js";
import type { UnavailableOperation } from "../../renderer/browser/disconnectedHost.js";
import type { IGitApi } from "../common/gitApi.js";

export function createDisconnectedGitApi(unavailable: UnavailableOperation): IGitApi {
	return {
		repositories: () => unavailable("git.repositories"),
		status: () => unavailable("git.status"),
		history: () => unavailable("git.history"),
		branches: () => unavailable("git.branches"),
		switchBranch: () => unavailable("git.switchBranch"),
		graph: () => unavailable("git.graph"),
		commitChanges: () => unavailable("git.commitChanges"),
		commitFile: () => unavailable("git.commitFile"),
		changeFile: () => unavailable("git.changeFile"),
		stage: () => unavailable("git.stage"),
		unstage: () => unavailable("git.unstage"),
		discardWorktree: () => unavailable("git.discardWorktree"),
		commit: () => unavailable("git.commit"),
		fetch: () => unavailable("git.fetch"),
		pull: () => unavailable("git.pull"),
		push: () => unavailable("git.push"),
	};
}

export function createViteDevGitApi(connection: ViteDevAppServerConnection): IGitApi {
	return {
		repositories: () => viteDevRequest(connection, "git/repositories", {}),
		status: (params) => viteDevRequest(connection, "git/status", params),
		history: (params) => viteDevRequest(connection, "git/history", params),
		branches: (params) => viteDevRequest(connection, "git/branch/list", params),
		switchBranch: (params) => viteDevRequest(connection, "git/branch/switch", params),
		graph: (params) => viteDevRequest(connection, "git/graph", params),
		commitChanges: (params) => viteDevRequest(connection, "git/commitChanges", params),
		commitFile: (params) => viteDevRequest(connection, "git/commitFile", params),
		changeFile: (params) => viteDevRequest(connection, "git/changeFile", params),
		stage: (params) => viteDevRequest(connection, "git/stage", params),
		unstage: (params) => viteDevRequest(connection, "git/unstage", params),
		discardWorktree: (params) => viteDevRequest(connection, "git/discardWorktree", params),
		commit: (params) => viteDevRequest(connection, "git/commit", params),
		fetch: (params) => viteDevRequest(connection, "git/fetch", params),
		pull: (params) => viteDevRequest(connection, "git/pull", params),
		push: (params) => viteDevRequest(connection, "git/push", params),
	};
}
