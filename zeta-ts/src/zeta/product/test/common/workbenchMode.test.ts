import assert from 'node:assert/strict';
import { mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import test from 'node:test';
import { resolveWorkbenchModeIdFromUrl, WorkbenchModeId, WorkbenchModeRegistry, withWorkbenchModeId } from '../../../product/common/workbenchMode.js';
import { readPersistedWorkbenchModeId } from '../../../product/node/product.js';

test('Workbench mode registry is the complete owner of built-in definitions', () => {
	assert.equal(WorkbenchModeRegistry.resolveModeId(undefined), WorkbenchModeId.Code);
	assert.equal(WorkbenchModeRegistry.resolveModeId(''), WorkbenchModeId.Code);
	assert.deepEqual(WorkbenchModeRegistry.modeIds, [WorkbenchModeId.Code, WorkbenchModeId.Academic]);
	assert.deepEqual(WorkbenchModeRegistry.definitions.map(({ id, label, storageNamespace }) => ({ id, label, storageNamespace })), [
		{ id: WorkbenchModeId.Code, label: 'Code', storageNamespace: 'code' },
		{ id: WorkbenchModeId.Academic, label: 'Academic', storageNamespace: 'academic' },
	]);
	assert.equal(WorkbenchModeRegistry.get(WorkbenchModeId.Code).dedicatedSessions?.rendererEntry, 'sessions-code');
	assert.equal(WorkbenchModeRegistry.get(WorkbenchModeId.Academic).dedicatedSessions, undefined);
});

test('Workbench mode URLs override the fallback without changing the renderer entry', () => {
	const codeUrl = 'file:///renderer/workbench/workbench.html';
	const academicUrl = withWorkbenchModeId(codeUrl, WorkbenchModeId.Academic);
	assert.equal(resolveWorkbenchModeIdFromUrl(codeUrl, WorkbenchModeId.Code), WorkbenchModeId.Code);
	assert.equal(resolveWorkbenchModeIdFromUrl(academicUrl, WorkbenchModeId.Code), WorkbenchModeId.Academic);
	assert.equal(new URL(academicUrl).pathname, new URL(codeUrl).pathname);
});

test('Workbench mode registry rejects unknown ids', () => {
	assert.throws(() => WorkbenchModeRegistry.resolveModeId('enterprise'), /Unknown Zeta Workbench mode 'enterprise'/);
});

test('persisted startup mode accepts only a registered id', () => {
	const directory = mkdtempSync(join(tmpdir(), 'zeta-mode-'));
	const filePath = join(directory, 'configuration.json');
	try {
		assert.equal(readPersistedWorkbenchModeId(filePath, WorkbenchModeId.Code), WorkbenchModeId.Code);
		writeFileSync(filePath, JSON.stringify({ version: 1, values: { 'workbench.mode': 'academic' } }));
		assert.equal(readPersistedWorkbenchModeId(filePath, WorkbenchModeId.Code), WorkbenchModeId.Academic);
		writeFileSync(filePath, JSON.stringify({ version: 1, values: { 'workbench.mode': 'unknown' } }));
		assert.equal(readPersistedWorkbenchModeId(filePath, WorkbenchModeId.Code), WorkbenchModeId.Code);
	} finally {
		rmSync(directory, { force: true, recursive: true });
	}
});
