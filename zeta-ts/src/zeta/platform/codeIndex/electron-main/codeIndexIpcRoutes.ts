import { APP_SERVER_METHODS, type ProviderConfigureParams, type SemanticCodeIndexAuthorizeParams, type SemanticCodeIndexConfigureParams } from "../../../../../generated/app-server/types.js";
import type { AppServerSupervisor } from "../../app-server/electron-main/app-server-supervisor.js";
import { nonEmptyString, nonNegativeInteger, positiveInteger, record } from "../../ipc/electron-main/ipcValidation.js";
import type { IpcRoute } from "../../ipc/electron-main/trustedIpcRouter.js";

export function codeIndexIpcRoutes(supervisor: AppServerSupervisor): readonly IpcRoute<unknown, unknown>[] {
	return [
		route({ channel: "zeta:code-index:config-read", validate: emptyParams, invoke: () => supervisor.request(APP_SERVER_METHODS["config/read"], {}) }),
		route({ channel: "zeta:code-index:provider-configure", validate: providerConfigureParams, invoke: params => supervisor.request(APP_SERVER_METHODS["provider/configure"], params) }),
		route({ channel: "zeta:code-index:semantic-configure", validate: configureParams, invoke: params => supervisor.request(APP_SERVER_METHODS["workspace/codeIndex/semantic/configure"], params) }),
		route({ channel: "zeta:code-index:semantic-authorize", validate: commandParams, invoke: params => supervisor.request(APP_SERVER_METHODS["workspace/codeIndex/semantic/authorize"], params) }),
		route({ channel: "zeta:code-index:semantic-revoke", validate: commandParams, invoke: params => supervisor.request(APP_SERVER_METHODS["workspace/codeIndex/semantic/revoke"], params) }),
		route({ channel: "zeta:code-index:status", validate: emptyParams, invoke: () => supervisor.request(APP_SERVER_METHODS["workspace/codeIndex/status"], {}) }),
		route({ channel: "zeta:code-index:semantic-cancel", validate: emptyParams, invoke: () => supervisor.request(APP_SERVER_METHODS["workspace/codeIndex/semantic/cancel"], {}) }),
		route({ channel: "zeta:code-index:semantic-retry", validate: emptyParams, invoke: () => supervisor.request(APP_SERVER_METHODS["workspace/codeIndex/semantic/retry"], {}) }),
	];
}

function providerConfigureParams(value: unknown): ProviderConfigureParams {
	const params = record(value, ["commandId", "expectedRevision", "config"]);
	const config = record(params.config, ["provider", "baseUrl", "maxOutputTokens", "modelContext"]);
	if (config.baseUrl !== null && config.baseUrl !== undefined && typeof config.baseUrl !== "string") throw new Error("config.baseUrl must be a string or null");
	if (config.maxOutputTokens !== null && config.maxOutputTokens !== undefined && !Number.isSafeInteger(config.maxOutputTokens)) throw new Error("config.maxOutputTokens must be an integer or null");
	const modelContext = providerModelContext(config.modelContext);
	return {
		commandId: nonEmptyString(params.commandId, "commandId"),
		expectedRevision: nonNegativeInteger(params.expectedRevision, "expectedRevision"),
		config: {
			provider: nonEmptyString(config.provider, "config.provider"),
			baseUrl: config.baseUrl as string | null | undefined,
			maxOutputTokens: config.maxOutputTokens as number | null | undefined,
			modelContext,
		},
	};
}

function providerModelContext(value: unknown): NonNullable<ProviderConfigureParams["config"]["modelContext"]> {
	if (typeof value !== "object" || value === null || Array.isArray(value)) throw new Error("config.modelContext must be an object");
	return Object.fromEntries(Object.entries(value).map(([model, raw]) => {
		if (model.trim().length === 0) throw new Error("config.modelContext model keys must not be empty");
		const context = record(raw, ["contextWindow"], ["autoCompactTokenLimit"]);
		const autoCompactTokenLimit = context.autoCompactTokenLimit === null || context.autoCompactTokenLimit === undefined
			? null
			: positiveInteger(context.autoCompactTokenLimit, `config.modelContext.${model}.autoCompactTokenLimit`);
		return [model, {
			contextWindow: positiveInteger(context.contextWindow, `config.modelContext.${model}.contextWindow`),
			autoCompactTokenLimit,
		}];
	}));
}

function route<P, R>(definition: IpcRoute<P, R>): IpcRoute<unknown, unknown> {
	return { channel: definition.channel, validate: definition.validate, invoke: params => definition.invoke(params as P) };
}

function emptyParams(value: unknown): Record<string, never> {
	if (value === undefined) return {};
	return record(value, []) as Record<string, never>;
}

function commandParams(value: unknown): SemanticCodeIndexAuthorizeParams {
	const params = record(value, ["commandId", "expectedRevision"]);
	return { commandId: nonEmptyString(params.commandId, "commandId"), expectedRevision: nonNegativeInteger(params.expectedRevision, "expectedRevision") };
}

function configureParams(value: unknown): SemanticCodeIndexConfigureParams {
	const params = record(value, ["commandId", "expectedRevision", "selection", "automaticContext"]);
	if (params.automaticContext !== "off" && params.automaticContext !== "firstInvocation") throw new Error("automaticContext must be off or firstInvocation");
	const automaticContext: SemanticCodeIndexConfigureParams["automaticContext"] = params.automaticContext;
	const command = { commandId: nonEmptyString(params.commandId, "commandId"), expectedRevision: nonNegativeInteger(params.expectedRevision, "expectedRevision"), automaticContext };
	const selection = record(params.selection, ["type"], ["models"]);
	if (selection.type === "disabled") return { ...command, selection: { type: "disabled" } };
	if (selection.type !== "remote") throw new Error("selection.type must be disabled or remote");
	const models = record(selection.models, ["embeddingModel"], ["rerankModel"]);
	return {
		...command,
		selection: {
			type: "remote",
			models: {
				embeddingModel: modelRef(models.embeddingModel, "embeddingModel"),
				rerankModel: models.rerankModel === null || models.rerankModel === undefined ? null : modelRef(models.rerankModel, "rerankModel"),
			},
		},
	};
}

function modelRef(value: unknown, field: string): { provider: string; model: string } {
	const ref = record(value, ["provider", "model"]);
	return { provider: nonEmptyString(ref.provider, `${field}.provider`), model: nonEmptyString(ref.model, `${field}.model`) };
}
