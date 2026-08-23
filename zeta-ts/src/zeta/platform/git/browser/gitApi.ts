import type { ViteDevAppServerConnection } from "../../app-server/browser/viteDevConnection.js";
import { viteDevRequest } from "../../app-server/browser/viteDevRequest.js";
import type { UnavailableOperation } from "../../renderer/browser/disconnectedHost.js";
import type { IGitApi } from "../common/gitApi.js";

export function createDisconnectedGitApi(unavailable: UnavailableOperation): IGitApi {
	return {
		status: () => unavailable("git.status"),
		history: () => unavailable("git.history"),
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
		status: () => viteDevRequest(connection, "git/status", {}),
		history: () => viteDevRequest(connection, "git/history", {}),
		graph: (params) => viteDevRequest(connection, "git/graph", params),
		commitChanges: (params) => viteDevRequest(connection, "git/commitChanges", params),
		commitFile: (params) => viteDevRequest(connection, "git/commitFile", params),
		changeFile: (params) => viteDevRequest(connection, "git/changeFile", params),
		stage: (params) => viteDevRequest(connection, "git/stage", params),
		unstage: (params) => viteDevRequest(connection, "git/unstage", params),
		discardWorktree: (params) => viteDevRequest(connection, "git/discardWorktree", params),
		commit: (params) => viteDevRequest(connection, "git/commit", params),
		fetch: () => viteDevRequest(connection, "git/fetch", {}),
		pull: () => viteDevRequest(connection, "git/pull", {}),
		push: () => viteDevRequest(connection, "git/push", {}),
	};
}
