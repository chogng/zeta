import { type IDiffComputationService } from '../../../../editor/common/diff/diffComputationService.js';
import { createServiceIdentifier } from '../../../../platform/instantiation/common/instantiation.js';

/** Workbench-owned factory for editor diff computations. */
export interface IDiffService {
	createComputationService(): IDiffComputationService;
}

export const IDiffService = createServiceIdentifier<IDiffService>('diffService');
