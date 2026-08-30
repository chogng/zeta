import type { ViteDevAppServerConnection } from "../../app-server/browser/viteDevConnection.js";
import { viteDevRequest } from "../../app-server/browser/viteDevRequest.js";
import type { UnavailableOperation } from "../../renderer/browser/disconnectedHost.js";
import type { IDirPermissionsApi } from "../common/dirPermissionsApi.js";

export function createDisconnectedDirPermissionsApi(unavailable: UnavailableOperation): IDirPermissionsApi {
	return {
		list: () => unavailable("dirPermissions.list"),
		read: () => unavailable("dirPermissions.read"),
		set: () => unavailable("dirPermissions.set"),
		forget: () => unavailable("dirPermissions.forget"),
	};
}
export function createViteDevDirPermissionsApi(connection: ViteDevAppServerConnection): IDirPermissionsApi {
	return {
		list: () => viteDevRequest(connection, "config/dirPermissions/list", {}),
		read: params => viteDevRequest(connection, "config/dirPermissions/read", params),
		set: params => viteDevRequest(connection, "config/dirPermissions/set", params),
		forget: params => viteDevRequest(connection, "config/dirPermissions/forget", params),
	};
}
