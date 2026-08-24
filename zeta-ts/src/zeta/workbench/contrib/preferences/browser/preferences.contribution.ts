import { IClipboardService } from '../../../../platform/clipboard/common/clipboardService.js';
import { IConfigurationService } from '../../../../platform/configuration/common/configurationService.js';
import { IContextMenuService } from '../../../../platform/contextview/browser/contextMenu.js';
import { IContextViewService } from '../../../../platform/contextview/browser/contextView.js';
import { ILayoutService } from '../../../../platform/layout/common/layoutService.js';
import { registerWorkbenchContribution, WorkbenchPhase } from '../../../common/contributions.js';
import { ILocalizationService } from '../../../services/localization/common/localizationService.js';
import { IPreferencesService } from '../../../services/preferences/common/preferences.js';
import { PreferencesEditor } from './preferencesEditor.js';
import './keyboardLayoutPicker.js';
import './keyboardShortcutsEditor.contribution.js';
import './preferencesActions.js';

registerWorkbenchContribution(
	'workbench.contrib.preferencesEditor',
	WorkbenchPhase.BlockStartup,
	accessor => new PreferencesEditor({
		clipboardService: accessor.get(IClipboardService),
		configurationService: accessor.get(IConfigurationService),
		container: accessor.get(ILayoutService).mainContainer,
		contextMenuProvider: accessor.get(IContextMenuService),
		contextViewProvider: accessor.get(IContextViewService),
		localizationService: accessor.get(ILocalizationService),
		preferencesService: accessor.get(IPreferencesService),
	}),
);
