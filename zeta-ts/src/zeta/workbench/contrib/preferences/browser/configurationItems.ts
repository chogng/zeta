import './media/configurationItems.css';
import type { IContextMenuProvider } from '../../../../base/browser/contextmenu.js';
import type { IContextViewProvider } from '../../../../base/browser/ui/contextview/contextview.js';
import { DisposableOwner } from '../../../../base/common/lifecycle.js';
import type { IClipboardService } from '../../../../platform/clipboard/common/clipboardService.js';
import type { IConfigurationService } from '../../../../platform/configuration/common/configurationService.js';
import type { ConfigurationSettingDescriptor, ConfigurationSettingsGroupDescriptor } from '../common/settingsDescriptors.js';
import type { SettingsItemContribution, SettingsItemView, SettingsSectionContribution } from './settingsContributions.js';
import { SettingsWidgets, type SettingsWidgetsPresentation } from './settingsWidgets.js';

export interface ConfigurationItemsContributionOptions {
	readonly clipboardService: IClipboardService;
	readonly configurationService: IConfigurationService;
	readonly contextMenuProvider: IContextMenuProvider;
	readonly contextViewProvider: IContextViewProvider;
	readonly groups: readonly ConfigurationSettingsGroupDescriptor[];
	readonly onStatus: (message: string, isError: boolean) => void;
	readonly presentation: SettingsWidgetsPresentation;
}

/** Contributes typed configuration items to the unified Settings layout. */
export class ConfigurationItemsContribution extends DisposableOwner implements SettingsSectionContribution {
	public readonly groups;
	private readonly widgets: SettingsWidgets;

	constructor(public readonly sectionId: string, document: Document, options: ConfigurationItemsContributionOptions) {
		super();
		this.widgets = this.own(new SettingsWidgets(document, {
			clipboardService: options.clipboardService,
			configurationService: options.configurationService,
			contextMenuProvider: options.contextMenuProvider,
			contextViewProvider: options.contextViewProvider,
			onStatus: options.onStatus,
			presentation: options.presentation,
		}));
		this.groups = options.groups.map(group => ({
			...group,
			settings: group.settings.map(setting => this.item(setting)),
		}));
	}

	private item(descriptor: ConfigurationSettingDescriptor): SettingsItemContribution {
		return {
			...descriptor,
			createView: () => new ConfigurationItemView(this.widgets, descriptor),
		};
	}
}

class ConfigurationItemView extends DisposableOwner implements SettingsItemView {
	public readonly element: HTMLElement;

	constructor(private readonly widgets: SettingsWidgets, descriptor: ConfigurationSettingDescriptor) {
		super();
		this.element = widgets.render(descriptor);
	}

	public dispose(): void {
		const id = this.element.dataset.settingsItemId;
		if (id) this.widgets.disposeItem(id);
	}
}
