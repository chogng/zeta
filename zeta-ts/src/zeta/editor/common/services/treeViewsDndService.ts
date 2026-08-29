import type { VSDataTransfer } from '../../../base/common/dataTransfer.js';
import { createServiceIdentifier, registerSingleton, type ServiceContainer } from '../../../platform/instantiation/common/instantiation.js';
import { type ITreeViewsDnDService as ITreeViewsDnDServiceCommon, TreeViewsDnDService } from './treeViewsDnd.js';

export interface ITreeViewsDnDService extends ITreeViewsDnDServiceCommon<VSDataTransfer> {}

export const ITreeViewsDnDService = createServiceIdentifier<ITreeViewsDnDService>('treeViewsDndService');
const treeViewsDnDServiceDescriptor = registerSingleton(ITreeViewsDnDService, () => new TreeViewsDnDService<VSDataTransfer>());

/** Registers the window-scoped tree drag transfer service. */
export function registerTreeViewsDnDService(container: ServiceContainer): void {
	container.registerSingletonDescriptor(treeViewsDnDServiceDescriptor);
}
