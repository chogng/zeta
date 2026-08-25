import { IClipboardService } from '../../../../platform/clipboard/common/clipboardService.js';
import { IConfigurationService } from '../../../../platform/configuration/common/configurationService.js';
import { IContextMenuService } from '../../../../platform/contextview/browser/contextMenu.js';
import { IContextViewService } from '../../../../platform/contextview/browser/contextView.js';
import { SyncDescriptor } from '../../../../platform/instantiation/common/instantiation.js';
import { EditorPaneMatch } from '../../../browser/parts/editor/editorPane.js';
import { registerEditorPane } from '../../../browser/parts/editor/editorRegistry.js';
import { ILocalizationService } from '../../../services/localization/common/localizationService.js';
import { isPreferencesEditorInput } from '../../../services/preferences/common/preferencesEditorInput.js';
import { PreferencesEditor, PreferencesEditorId } from './preferencesEditor.js';
import { registerPreferencesEditorPane } from './preferencesEditorRegistry.js';
import { SettingsEditorPane, SettingsEditorPaneId } from './settingsEditor.js';
import './keyboardLayoutPicker.js';
import './keyboardShortcutsEditor.contribution.js';
import './preferencesActions.js';

registerPreferencesEditorPane({
	id: SettingsEditorPaneId,
	title: 'Settings',
	order: 1,
	ctorDescriptor: new SyncDescriptor(SettingsEditorPane, {
		serviceDependencies: [
			IClipboardService,
			IConfigurationService,
			IContextMenuService,
			IContextViewService,
			ILocalizationService,
		],
	}),
});

registerEditorPane({
	id: PreferencesEditorId,
	name: 'Preferences',
	canOpen: input => isPreferencesEditorInput(input) ? EditorPaneMatch.Default : EditorPaneMatch.None,
	create: options => {
		if (!options.instantiationService) throw new Error('Preferences editor requires the Instantiation Service');
		return new PreferencesEditor(options.instantiationService, options.instantiationService.get(ILocalizationService));
	},
});
