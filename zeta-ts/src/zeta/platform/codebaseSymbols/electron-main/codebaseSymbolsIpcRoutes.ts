import { APP_SERVER_METHODS, type CodebaseSymbolsSearchParams, type DocumentOverlayCloseParams, type DocumentOverlaySynchronizeParams } from "../../../../../generated/app-server/types.js";
import { VSBuffer } from "../../../base/common/buffer.js";
import type { AppServerSupervisor } from "../../app-server/electron-main/app-server-supervisor.js";
import { positiveInteger, record } from "../../ipc/electron-main/ipcValidation.js";
import type { IpcRoute } from "../../ipc/electron-main/trustedIpcRouter.js";

export function codebaseSymbolsIpcRoutes(supervisor: AppServerSupervisor): readonly IpcRoute<unknown, unknown>[] {
	return [
		route({ channel: "zeta:codebase-symbols:status", validate: emptyParams, invoke: () => supervisor.request(APP_SERVER_METHODS["codebase/symbols/status"], {}) }),
		route({ channel: "zeta:codebase-symbols:search", validate: searchParams, invoke: params => supervisor.request(APP_SERVER_METHODS["codebase/symbols/search"], params) }),
		route({ channel: "zeta:codebase-symbols:document-synchronize", validate: synchronizeParams, invoke: params => supervisor.request(APP_SERVER_METHODS["codeIntelligence/document/synchronize"], params) }),
		route({ channel: "zeta:codebase-symbols:document-close", validate: closeParams, invoke: params => supervisor.request(APP_SERVER_METHODS["codeIntelligence/document/close"], params) }),
	];
}

function route<P, R>(definition: IpcRoute<P, R>): IpcRoute<unknown, unknown> {
	return { channel: definition.channel, validate: definition.validate, invoke: params => definition.invoke(params as P) };
}

function emptyParams(value: unknown): Record<string, never> {
	if (value === undefined) return {};
	return record(value, []) as Record<string, never>;
}

function searchParams(value: unknown): CodebaseSymbolsSearchParams {
	const params = record(value, ["query", "maxResults"]);
	if (typeof params.query !== "string" || VSBuffer.fromString(params.query).byteLength > 8192) throw new Error("query must be a string no larger than 8192 UTF-8 bytes");
	const maxResults = positiveInteger(params.maxResults, "maxResults");
	if (maxResults > 100) throw new Error("maxResults must not exceed 100");
	return { query: params.query, maxResults };
}

function synchronizeParams(value: unknown): DocumentOverlaySynchronizeParams {
	const params = record(value, ["document"]);
	const document = record(params.document, ["path", "languageId", "revision", "text"]);
	if (typeof document.path !== "string" || document.path.length === 0 || document.path.length > 4096) throw new Error("document.path must be a bounded path");
	if (typeof document.languageId !== "string" || document.languageId.length === 0 || document.languageId.length > 128) throw new Error("document.languageId must be bounded text");
	if (!Number.isSafeInteger(document.revision) || (document.revision as number) < 1) throw new Error("document.revision must be a positive safe integer");
	if (typeof document.text !== "string" || VSBuffer.fromString(document.text).byteLength > 10 * 1024 * 1024) throw new Error("document.text exceeds its transport limit");
	return { document: { path: document.path, languageId: document.languageId, revision: document.revision as number, text: document.text } };
}

function closeParams(value: unknown): DocumentOverlayCloseParams {
	const params = record(value, ["path"]);
	if (typeof params.path !== "string" || params.path.length === 0 || params.path.length > 4096) throw new Error("path must be a bounded path");
	return { path: params.path };
}
