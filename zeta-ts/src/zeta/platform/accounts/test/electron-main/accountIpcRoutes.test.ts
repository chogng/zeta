import { strict as assert } from 'node:assert';
import test from 'node:test';
import type { AppServerSupervisor } from '../../../../platform/app-server/electron-main/app-server-supervisor.js';
import { accountIpcRoutes } from '../../../../platform/accounts/electron-main/accountIpcRoutes.js';

test('account IPC accepts supported login methods and opens a device challenge', async () => {
	const requests: unknown[] = [];
	const opened: string[] = [];
	const copied: string[] = [];
	const supervisor = {
		request: async (method: unknown, params: unknown) => {
			requests.push([method, params]);
			return { type: 'deviceCode', loginId: 'login-1', verificationUrl: 'https://auth.kimi.com/device', userCode: 'KIMI-CODE' };
		},
	} as unknown as AppServerSupervisor;
	const route = accountIpcRoutes(supervisor, {
		openerService: { openExternal: async target => { opened.push(target); } },
		clipboardService: { writeText: async value => { copied.push(value); } },
	}).find(candidate => candidate.channel === 'zeta:accounts:login-start');
	assert.ok(route);

	const result = await route.invoke(route.validate({ method: { type: 'kimiDeviceCode' } }));
	assert.deepEqual(result, { type: 'deviceCode', loginId: 'login-1', verificationUrl: 'https://auth.kimi.com/device', userCode: 'KIMI-CODE' });
	assert.deepEqual(opened, ['https://auth.kimi.com/device']);
	assert.deepEqual(copied, ['KIMI-CODE']);
	assert.deepEqual(requests, [[{ method: 'account/login/start' }, { method: { type: 'kimiDeviceCode' } }]]);
	assert.deepEqual(route.validate({ method: { type: 'openAiChatGptBrowser' } }), { method: { type: 'openAiChatGptBrowser' } });
	assert.deepEqual(route.validate({ method: { type: 'openAiChatGptDeviceCode' } }), { method: { type: 'openAiChatGptDeviceCode' } });
	assert.throws(() => route.validate({ method: { type: 'apiKey' } }), /unsupported account login method/);
	assert.throws(() => route.validate({ method: { type: 'kimiDeviceCode' }, extra: true }), /exactly required keys/);
});

test('account IPC opens browser login without copying a device code', async () => {
	const opened: string[] = [];
	const copied: string[] = [];
	const supervisor = {
		request: async () => ({ type: 'browser', loginId: 'login-browser', authorizationUrl: 'https://auth.openai.com/authorize' }),
	} as unknown as AppServerSupervisor;
	const route = accountIpcRoutes(supervisor, {
		openerService: { openExternal: async target => { opened.push(target); } },
		clipboardService: { writeText: async value => { copied.push(value); } },
	}).find(candidate => candidate.channel === 'zeta:accounts:login-start');
	assert.ok(route);

	const result = await route.invoke(route.validate({ method: { type: 'openAiChatGptBrowser' } }));
	assert.deepEqual(result, { type: 'browser', loginId: 'login-browser', authorizationUrl: 'https://auth.openai.com/authorize' });
	assert.deepEqual(opened, ['https://auth.openai.com/authorize']);
	assert.deepEqual(copied, []);
});

test('account IPC cancels a device flow when opening the browser fails', async () => {
	const requests: unknown[] = [];
	const supervisor = {
		request: async (method: { method: string }, params: unknown) => {
			requests.push([method, params]);
			if (method.method === 'account/login/start') return { type: 'deviceCode', loginId: 'login-2', verificationUrl: 'https://auth.kimi.com/device', userCode: 'KIMI-CODE' };
			return { status: 'cancelled' };
		},
	} as unknown as AppServerSupervisor;
	const route = accountIpcRoutes(supervisor, {
		openerService: { openExternal: async () => { throw new Error('open failed'); } },
		clipboardService: { writeText: async () => {} },
	}).find(candidate => candidate.channel === 'zeta:accounts:login-start');
	assert.ok(route);

	await assert.rejects(Promise.resolve(route.invoke(route.validate({ method: { type: 'kimiDeviceCode' } }))), /open failed/);
	assert.deepEqual(requests, [
		[{ method: 'account/login/start' }, { method: { type: 'kimiDeviceCode' } }],
		[{ method: 'account/login/cancel' }, { loginId: 'login-2' }],
	]);
});
