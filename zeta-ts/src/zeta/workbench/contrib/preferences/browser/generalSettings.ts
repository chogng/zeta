import './media/generalSettings.css';
import type { IContextMenuProvider } from '../../../../base/browser/contextmenu.js';
import type { IContextViewProvider } from '../../../../base/browser/ui/contextview/contextview.js';
import type { IClipboardService } from '../../../../platform/clipboard/common/clipboardService.js';
import type { IConfigurationService } from '../../../../platform/configuration/common/configurationService.js';
import { AccessibilityConfiguration, type AccessibilityReductionConfiguration, type AccessibilitySupportConfiguration } from '../../../../platform/accessibility/common/accessibility.js';
import { HoverConfiguration, MaximumHoverDelay, MinimumHoverDelay } from '../../../../platform/hover/common/hoverService.js';
import type { WorkbenchModeId } from '../../../../product/common/workbenchMode.js';
import { WorkbenchConfiguration } from '../../../common/configuration.js';
import type { IWorkbenchModeService } from '../../../services/workbenchMode/common/workbenchModeService.js';
import type { IPreferencesService } from '../../../services/preferences/common/preferences.js';
import { MaximumSashHoverDelay, MaximumSashSize, MinimumSashHoverDelay, MinimumSashSize, SashConfiguration } from '../../sash/common/sash.js';
import { actionSetting, booleanSetting, boundSelectSetting, type ConfigurationSettingsGroupDescriptor, numberSetting, selectSetting } from '../common/settingsDescriptors.js';
import { ConfigurationSettingsPane } from './configurationSettings.js';

interface GeneralSettingsPaneOptions {
	readonly clipboardService: IClipboardService;
	readonly configurationService: IConfigurationService;
	readonly contextMenuProvider: IContextMenuProvider;
	readonly contextViewProvider: IContextViewProvider;
	readonly workbenchModeService: IWorkbenchModeService;
	readonly preferencesService: IPreferencesService;
}

/** Core application preferences declared independently of their browser controls. */
export class GeneralSettingsPane extends ConfigurationSettingsPane {
	constructor(container: HTMLElement, options: GeneralSettingsPaneOptions) {
		super(container, {
			clipboardService: options.clipboardService,
			configurationService: options.configurationService,
			contextMenuProvider: options.contextMenuProvider,
			contextViewProvider: options.contextViewProvider,
			groups: generalSettingsGroups(options.workbenchModeService, options.preferencesService),
			presentation: 'general',
		});
	}
}

function generalSettingsGroups(workbenchModeService: IWorkbenchModeService, preferencesService: IPreferencesService): readonly ConfigurationSettingsGroupDescriptor[] {
	return [
		{
			id: 'mode',
			title: 'Workbench Mode',
			description: 'Choose the capabilities assembled for this window.',
			settings: [boundSelectSetting<WorkbenchModeId>({
				id: WorkbenchConfiguration.mode.key,
				defaultValue: WorkbenchConfiguration.mode.defaultValue,
				getValue: () => workbenchModeService.currentModeId,
				updateValue: modeId => workbenchModeService.switchMode(modeId),
				resetValue: () => workbenchModeService.resetMode(),
			}, 'Workbench mode', 'Switch the capability assembly for this window. The current window reloads after a change.', workbenchModeService.availableModes.map(({ id, label }) => ({ value: id, label })))],
		},
		{
			id: 'keyboard',
			title: 'Keyboard',
			description: 'Customize command shortcuts for this profile.',
			settings: [actionSetting(
				'workbench.keyboardShortcuts',
				'Keyboard Shortcuts',
				'Open the Keyboard Shortcuts Editor in a Workbench tab.',
				'Open Keyboard Shortcuts',
				() => preferencesService.openKeybindings(),
			)],
		},
		{
			id: 'accessibility',
			title: 'Accessibility',
			description: 'Adapt interaction and presentation to accessibility needs.',
			settings: [
				selectSetting(AccessibilityConfiguration.editorAccessibilitySupport, 'Screen reader optimization', 'Let the operating system decide, or explicitly enable or disable optimized editor accessibility behavior.', triStateOptions<AccessibilitySupportConfiguration>()),
				selectSetting(AccessibilityConfiguration.reduceMotion, 'Reduce motion', 'Limit non-essential animation throughout the Workbench.', triStateOptions<AccessibilityReductionConfiguration>()),
				selectSetting(AccessibilityConfiguration.reduceTransparency, 'Reduce transparency', 'Prefer opaque surfaces where the active theme supports them.', triStateOptions<AccessibilityReductionConfiguration>()),
				booleanSetting(AccessibilityConfiguration.underlineLinks, 'Always underline links', 'Keep link affordances visible without requiring hover or focus.'),
			],
		},
		{
			id: 'interaction',
			title: 'Interaction',
			description: 'Tune common pointer feedback and resize affordances.',
			settings: [
				numberSetting(HoverConfiguration.delay, 'Hover delay', 'Milliseconds before standard managed hovers appear.', MinimumHoverDelay, MaximumHoverDelay),
				numberSetting(HoverConfiguration.reducedDelay, 'Fast hover delay', 'Milliseconds used for controls that request reduced-delay hover feedback.', MinimumHoverDelay, MaximumHoverDelay),
				numberSetting(SashConfiguration.size, 'Resize handle size', 'Width in pixels of Workbench resize handles.', MinimumSashSize, MaximumSashSize),
				numberSetting(SashConfiguration.hoverDelay, 'Resize handle hover delay', 'Milliseconds before resize handles show hover feedback.', MinimumSashHoverDelay, MaximumSashHoverDelay),
			],
		},
	];
}

function triStateOptions<T extends AccessibilitySupportConfiguration | AccessibilityReductionConfiguration>(): readonly { readonly value: T; readonly label: string }[] {
	return [
		{ value: 'auto' as T, label: 'Auto' },
		{ value: 'on' as T, label: 'On' },
		{ value: 'off' as T, label: 'Off' },
	];
}
