import { type IDiffApi } from '../../../../platform/diff/common/diffApi.js';
import { AppServerDiffComputationService } from './appServerDiffComputationService.js';
import { type IDiffService } from '../common/diffService.js';

/** Creates Workbench diff computations backed by the App Server API. */
export class AppServerDiffService implements IDiffService {
	constructor(private readonly api: IDiffApi) {}

	createComputationService(): AppServerDiffComputationService {
		return new AppServerDiffComputationService(this.api);
	}
}
