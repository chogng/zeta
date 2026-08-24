import { Emitter } from '../../../../base/common/event.js';
import { DisposableOwner } from '../../../../base/common/lifecycle.js';
import type { IEditorService } from '../../editor/common/editorService.js';
import type { IPreferencesService } from '../common/preferences.js';
import { createKeyboardShortcutsEditorInput } from './keybindingsEditorInput.js';

/** Owns the browser Preferences entry points and window-scoped Settings state. */
export class PreferencesService extends DisposableOwner implements IPreferencesService {
	private readonly settingsVisibilityEmitter = this.own(new Emitter<boolean>());
	private readonly settingsSectionEmitter = this.own(new Emitter<string>());
	private settingsOpen = false;
	private settingsSectionId = 'general';

	public readonly onDidChangeSettingsVisibility = this.settingsVisibilityEmitter.event;
	public readonly onDidChangeSettingsSection = this.settingsSectionEmitter.event;

	constructor(private readonly resolveEditorService?: () => IEditorService) {
		super();
	}

	public get isSettingsOpen(): boolean {
		return this.settingsOpen;
	}

	public get activeSettingsSectionId(): string {
		return this.settingsSectionId;
	}

	public openSettings(sectionId?: string): void {
		if (sectionId !== undefined && sectionId !== this.settingsSectionId) {
			if (!sectionId) throw new TypeError('Settings section ID must not be empty');
			this.settingsSectionId = sectionId;
			this.settingsSectionEmitter.fire(sectionId);
		}
		if (this.settingsOpen) return;
		this.settingsOpen = true;
		this.settingsVisibilityEmitter.fire(true);
	}

	public closeSettings(): void {
		if (!this.settingsOpen) return;
		this.settingsOpen = false;
		this.settingsVisibilityEmitter.fire(false);
	}

	public async openKeybindings(): Promise<void> {
		if (!this.resolveEditorService) throw new Error('Keyboard Shortcuts editor is unavailable.');
		this.closeSettings();
		await this.resolveEditorService().openEditor(createKeyboardShortcutsEditorInput(), { pinned: true });
	}
}
