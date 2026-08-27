import { APP_SERVER_METHODS, type AppServerMethod, type MethodParams, type MethodResult } from "../../../../../generated/app-server/types.js";
import type { ViteDevAppServerConnection, ViteDevRequestOptions } from "./viteDevConnection.js";

export function viteDevRequest<M extends AppServerMethod>(connection: ViteDevAppServerConnection, method: M, params: MethodParams<M>, options?: ViteDevRequestOptions): Promise<MethodResult<M>> {
	return connection.request(APP_SERVER_METHODS[method], params, options);
}

export function voidResult<T>(promise: Promise<T>): Promise<void> {
	return promise.then(() => undefined);
}
