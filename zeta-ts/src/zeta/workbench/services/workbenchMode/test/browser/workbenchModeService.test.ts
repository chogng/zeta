import assert from 'node:assert/strict';
import test from 'node:test';
import type { ILifecycleService, ShutdownReason } from '../../../../../platform/lifecycle/common/lifecycleService.js';
import { WorkbenchModeId } from '../../../../../product/common/workbenchMode.js';
import { WorkbenchConfiguration } from '../../../../common/configuration.js';
import { WorkbenchConfigurationService } from '../../../configuration/browser/configurationService.js';
import { WorkbenchModeService } from '../../browser/workbenchModeService.js';

test('Workbench mode switch persists the id before shutdown and host reload', async () => {
	using configuration = new WorkbenchConfigurationService();
	const actions: string[] = [];
	const lifecycle = lifecycleService(reason => actions.push(`shutdown:${reason}`));
	using service = new WorkbenchModeService({
		currentModeId: WorkbenchModeId.Code,
		configurationService: configuration,
		lifecycleService: lifecycle,
		switchHostMode: async modeId => { actions.push(`host:${modeId}`); },
	});

	await service.switchMode(WorkbenchModeId.Academic);

	assert.equal(configuration.getValue(WorkbenchConfiguration.mode), WorkbenchModeId.Academic);
	assert.deepEqual(actions, ['shutdown:reload', 'host:academic']);
});

test('Workbench mode options come from the canonical registry', () => {
	using configuration = new WorkbenchConfigurationService();
	using service = new WorkbenchModeService({
		currentModeId: WorkbenchModeId.Code,
		configurationService: configuration,
		lifecycleService: lifecycleService(() => undefined),
		switchHostMode: async () => undefined,
	});

	assert.deepEqual(service.availableModes, [
		{ id: WorkbenchModeId.Code, label: 'Code' },
		{ id: WorkbenchModeId.Academic, label: 'Academic' },
	]);
});

test('selecting the active Workbench mode is a no-op', async () => {
	using configuration = new WorkbenchConfigurationService();
	const actions: string[] = [];
	using service = new WorkbenchModeService({
		currentModeId: WorkbenchModeId.Academic,
		configurationService: configuration,
		lifecycleService: lifecycleService(reason => actions.push(`shutdown:${reason}`)),
		switchHostMode: async modeId => { actions.push(`host:${modeId}`); },
	});

	await service.switchMode(WorkbenchModeId.Academic);

	assert.deepEqual(actions, []);
});

function lifecycleService(onShutdown: (reason: ShutdownReason) => void): ILifecycleService {
	return {
		phase: 'running',
		onWillShutdown: () => ({ dispose() {}, [Symbol.dispose]() {} }),
		onDidShutdown: () => ({ dispose() {}, [Symbol.dispose]() {} }),
		async shutdown(reason): Promise<void> { onShutdown(reason); },
	};
}
