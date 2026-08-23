import { DisposableOwner } from '../../../../base/common/lifecycle.js';
import type { IConfigurationService } from '../../../../platform/configuration/common/configurationService.js';
import type { ILifecycleService } from '../../../../platform/lifecycle/common/lifecycleService.js';
import { WorkbenchModeRegistry, type WorkbenchModeId } from '../../../../product/common/workbenchMode.js';
import { WorkbenchConfiguration } from '../../../common/configuration.js';
import type { IWorkbenchModeService, WorkbenchModeOption } from '../common/workbenchModeService.js';

export interface WorkbenchModeServiceOptions {
	readonly currentModeId: WorkbenchModeId;
	readonly configurationService: IConfigurationService;
	readonly lifecycleService: ILifecycleService;
	readonly switchHostMode: (modeId: WorkbenchModeId) => Promise<void>;
}

/** Persists a mode choice, flushes the Workbench, and asks its host to reload the window. */
export class WorkbenchModeService extends DisposableOwner implements IWorkbenchModeService {
	public readonly currentModeId: WorkbenchModeId;
	public readonly availableModes: readonly WorkbenchModeOption[] = Object.freeze(WorkbenchModeRegistry.definitions.map(({ id, label }) => Object.freeze({ id, label })));
	private readonly configurationService: IConfigurationService;
	private readonly lifecycleService: ILifecycleService;
	private readonly switchHostMode: (modeId: WorkbenchModeId) => Promise<void>;
	private switchOperation: Promise<void> | undefined;

	constructor(options: WorkbenchModeServiceOptions) {
		super();
		this.currentModeId = options.currentModeId;
		this.configurationService = options.configurationService;
		this.lifecycleService = options.lifecycleService;
		this.switchHostMode = options.switchHostMode;
	}

	public switchMode(modeId: WorkbenchModeId): Promise<void> {
		if (modeId === this.currentModeId) return Promise.resolve();
		this.switchOperation ??= this.performSwitch(modeId).catch(error => {
			this.switchOperation = undefined;
			throw error;
		});
		return this.switchOperation;
	}

	private async performSwitch(modeId: WorkbenchModeId): Promise<void> {
		await this.configurationService.updateValue(WorkbenchConfiguration.mode, modeId);
		await this.lifecycleService.shutdown('reload');
		await this.switchHostMode(modeId);
	}
}
