import { APP_SERVER_METHODS, type AccountLoginCancelParams, type AccountLoginStartParams, type AccountLoginStartResult, type AccountLogoutParams } from '../../../../../generated/app-server/types.js';
import type { AppServerSupervisor } from '../../app-server/electron-main/app-server-supervisor.js';
import { nonEmptyString, record } from '../../ipc/electron-main/ipcValidation.js';
import type { IpcRoute } from '../../ipc/electron-main/trustedIpcRouter.js';
import type { IClipboardService } from '../../clipboard/common/clipboardService.js';
import type { IOpenerService } from '../../opener/common/openerService.js';

export interface AccountLoginHostServices {
	readonly openerService: IOpenerService;
	readonly clipboardService: IClipboardService;
}

/** Exact-shape IPC routes for provider account authentication. */
export function accountIpcRoutes(supervisor: AppServerSupervisor, hostServices: AccountLoginHostServices): readonly IpcRoute<unknown, unknown>[] {
	return [
		route({ channel: 'zeta:accounts:read', validate: emptyParams, invoke: () => supervisor.request(APP_SERVER_METHODS['account/read'], {}) }),
		route({ channel: 'zeta:accounts:login-start', validate: loginStartParams, invoke: params => startLogin(supervisor, params, hostServices) }),
		route({ channel: 'zeta:accounts:login-cancel', validate: loginCancelParams, invoke: params => supervisor.request(APP_SERVER_METHODS['account/login/cancel'], params) }),
		route({ channel: 'zeta:accounts:logout', validate: logoutParams, invoke: params => supervisor.request(APP_SERVER_METHODS['account/logout'], params) }),
	];
}

async function startLogin(supervisor: AppServerSupervisor, params: AccountLoginStartParams, hostServices: AccountLoginHostServices): Promise<AccountLoginStartResult> {
	const started = await supervisor.request(APP_SERVER_METHODS['account/login/start'], params);
	try {
		await hostServices.openerService.openExternal(started.type === 'browser' ? started.authorizationUrl : started.verificationUrl);
		if (started.type === 'deviceCode') await hostServices.clipboardService.writeText(started.userCode);
		return started;
	} catch (error) {
		await supervisor.request(APP_SERVER_METHODS['account/login/cancel'], { loginId: started.loginId }).catch(() => undefined);
		throw error;
	}
}

function route<P, R>(definition: IpcRoute<P, R>): IpcRoute<unknown, unknown> {
	return {
		channel: definition.channel,
		validate: definition.validate,
		invoke: params => definition.invoke(params as P),
	};
}

function emptyParams(value: unknown): Record<string, never> {
	if (value === undefined) return {};
	return record(value, []) as Record<string, never>;
}

function loginStartParams(value: unknown): AccountLoginStartParams {
	const params = record(value, ['method']);
	const method = record(params.method, ['type']);
	switch (method.type) {
		case 'openAiChatGptBrowser': return { method: { type: 'openAiChatGptBrowser' } };
		case 'openAiChatGptDeviceCode': return { method: { type: 'openAiChatGptDeviceCode' } };
		case 'kimiDeviceCode': return { method: { type: 'kimiDeviceCode' } };
		default: throw new Error('unsupported account login method');
	}
}

function loginCancelParams(value: unknown): AccountLoginCancelParams {
	const params = record(value, ['loginId']);
	return { loginId: nonEmptyString(params.loginId, 'loginId') };
}

function logoutParams(value: unknown): AccountLogoutParams {
	const params = record(value, ['provider']);
	return { provider: nonEmptyString(params.provider, 'provider') };
}
