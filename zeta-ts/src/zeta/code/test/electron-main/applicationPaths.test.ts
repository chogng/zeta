import assert from 'node:assert/strict';
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import test from 'node:test';
import { ZetaRendererDirectory } from '../../common/application.js';
import { resolveApplicationDataPaths, resolvePackagedRendererRoot } from '../../electron-main/applicationPaths.js';

test('Desktop data paths use the single Zeta application identity', () => {
	assert.deepEqual(resolveApplicationDataPaths('/application-data'), {
		userDataPath: '/application-data/Zeta',
		sessionDataPath: '/application-data/Zeta/session-data',
	});
});

test('packaged renderer contains the shared Workbench and Code Sessions entry', () => {
	const rendererRoot = mkdtempSync(join(tmpdir(), 'zeta-renderer-'));
	try {
		assert.throws(() => resolvePackagedRendererRoot(rendererRoot), /renderer is incomplete/);
		const packagedRoot = join(rendererRoot, ZetaRendererDirectory);
		const workbenchRoot = join(packagedRoot, 'electron-browser', 'workbench');
		const sessionsRoot = join(packagedRoot, 'electron-browser', 'sessions');
		mkdirSync(workbenchRoot, { recursive: true });
		writeFileSync(join(workbenchRoot, 'workbench.html'), '<!doctype html>');
		assert.throws(() => resolvePackagedRendererRoot(rendererRoot), /sessions-code\.html/);
		mkdirSync(sessionsRoot, { recursive: true });
		writeFileSync(join(sessionsRoot, 'sessions-code.html'), '<!doctype html>');
		assert.equal(resolvePackagedRendererRoot(rendererRoot), packagedRoot);
	} finally {
		rmSync(rendererRoot, { force: true, recursive: true });
	}
});
