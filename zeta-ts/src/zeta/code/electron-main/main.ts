import { app } from 'electron/main';
import { join } from 'node:path';
import { WorkbenchModeRegistry } from '../../workbench/common/workbenchMode.js';
import { localProfileRoot } from '../../platform/profile/node/localProfile.js';
import { readPersistedWorkbenchModeId } from './readPersistedWorkbenchMode.js';
import { startElectronApplication } from './startElectronApplication.js';

const configuredModeId = !app.isPackaged && process.env.ZETA_WORKBENCH_MODE !== undefined
	? WorkbenchModeRegistry.resolveModeId(process.env.ZETA_WORKBENCH_MODE)
	: readPersistedWorkbenchModeId(join(localProfileRoot(), 'configuration.json'), WorkbenchModeRegistry.defaultModeId);

startElectronApplication({
	initialModeId: configuredModeId,
});
