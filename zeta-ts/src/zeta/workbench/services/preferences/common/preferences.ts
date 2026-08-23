import { createServiceIdentifier } from '../../../../platform/instantiation/common/instantiation.js';

/** Workbench-level entry point for opening Preferences surfaces. */
export interface IPreferencesService {
	openSettings(sectionId?: string): void;
	openKeybindings(): Promise<void>;
}

export const IPreferencesService = createServiceIdentifier<IPreferencesService>('preferencesService');
