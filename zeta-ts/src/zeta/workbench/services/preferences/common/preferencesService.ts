import type { IPreferencesService } from './preferences.js';
import type { ISettingsService } from './settings.js';
import type { IEditorService } from '../../editor/common/editorService.js';
import { createKeyboardShortcutsEditorInput } from './keybindingsEditorInput.js';

/** Opens Preferences surfaces while their focused services retain UI state. */
export class PreferencesService implements IPreferencesService {
	constructor(
		private readonly settingsService: ISettingsService,
		private readonly resolveEditorService?: () => IEditorService,
	) {}

	public openSettings(sectionId?: string): void {
		this.settingsService.open(sectionId);
	}

	public async openKeybindings(): Promise<void> {
		if (!this.resolveEditorService) throw new Error('Keyboard Shortcuts editor is unavailable.');
		this.settingsService.close();
		await this.resolveEditorService().openEditor(createKeyboardShortcutsEditorInput(), { pinned: true });
	}
}
