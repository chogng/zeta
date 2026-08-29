import type { ViteDevAppServerConnection } from "../../app-server/browser/viteDevConnection.js";
import { viteDevRequest } from "../../app-server/browser/viteDevRequest.js";
import type { UnavailableOperation } from "../../renderer/browser/disconnectedHost.js";
import type { ITurnChangesApi } from "../common/turnChangesApi.js";

export function createDisconnectedTurnChangesApi(unavailable: UnavailableOperation): ITurnChangesApi {
	return {
		list: () => unavailable("turnChanges.list"),
		read: () => unavailable("turnChanges.read"),
		readFile: () => unavailable("turnChanges.readFile"),
		generateMessage: () => unavailable("turnChanges.generateMessage"),
		updateDraft: () => unavailable("turnChanges.updateDraft"),
		commit: () => unavailable("turnChanges.commit"),
		discardThread: () => unavailable("turnChanges.discardThread"),
	};
}

export function createViteDevTurnChangesApi(connection: ViteDevAppServerConnection): ITurnChangesApi {
	return {
		list: (params) => viteDevRequest(connection, "turnChanges/list", params),
		read: (params) => viteDevRequest(connection, "turnChanges/read", params),
		readFile: (params) => viteDevRequest(connection, "turnChanges/readFile", params),
		generateMessage: (params) => viteDevRequest(connection, "turnChanges/generateMessage", params),
		updateDraft: (params) => viteDevRequest(connection, "turnChanges/updateDraft", params),
		commit: (params) => viteDevRequest(connection, "turnChanges/commit", params),
		discardThread: (params) => viteDevRequest(connection, "turnChanges/discardThread", params),
	};
}
