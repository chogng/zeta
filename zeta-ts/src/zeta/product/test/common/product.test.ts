import assert from 'node:assert/strict';
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import test from 'node:test';
import { ZetaDesktopApplication } from '../../../product/common/product.js';
import { resolveApplicationDataPaths, resolvePackagedRendererRoot } from '../../../product/node/product.js';

test('Desktop data paths are owned by the shared application identity', () => {
	const paths = resolveApplicationDataPaths('/application-data', ZetaDesktopApplication);
	assert.deepEqual(paths, {
		userDataPath: '/application-data/Zeta',
		sessionDataPath: '/application-data/Zeta/session-data',
	});
});

test('packaged renderer contains the shared Workbench and Code Sessions entry', () => {
	const rendererRoot = mkdtempSync(join(tmpdir(), 'zeta-renderer-'));
	try {
		assert.throws(() => resolvePackagedRendererRoot(rendererRoot, ZetaDesktopApplication), /renderer is incomplete/);
		const packagedRoot = join(rendererRoot, ZetaDesktopApplication.rendererDirectory);
		const workbenchRoot = join(packagedRoot, 'electron-browser', 'workbench');
		const sessionsRoot = join(packagedRoot, 'electron-browser', 'sessions');
		mkdirSync(workbenchRoot, { recursive: true });
		writeFileSync(join(workbenchRoot, 'workbench.html'), '<!doctype html>');
		assert.throws(() => resolvePackagedRendererRoot(rendererRoot, ZetaDesktopApplication), /sessions-code\.html/);
		mkdirSync(sessionsRoot, { recursive: true });
		writeFileSync(join(sessionsRoot, 'sessions-code.html'), '<!doctype html>');
		assert.equal(resolvePackagedRendererRoot(rendererRoot, ZetaDesktopApplication), packagedRoot);
	} finally {
		rmSync(rendererRoot, { force: true, recursive: true });
	}
});
