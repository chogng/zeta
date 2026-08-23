import type { IContextMenuProvider } from '../../../../base/browser/contextmenu.js';
import { h } from '../../../../base/browser/dom.js';
import type { IContextViewProvider } from '../../../../base/browser/ui/contextview/contextview.js';
import { DisposableOwner } from '../../../../base/common/lifecycle.js';
import type { IClipboardService } from '../../../../platform/clipboard/common/clipboardService.js';
import type { IConfigurationService } from '../../../../platform/configuration/common/configurationService.js';
import type { ConfigurationSettingDescriptor, ConfigurationSettingsGroupDescriptor } from '../common/settingsDescriptors.js';
import { SettingsLayout } from './settingsLayout.js';
import { SettingsTree } from './settingsTree.js';
import { SettingsTreeModel } from './settingsTreeModels.js';
import { SettingsWidgets, type SettingsWidgetsPresentation } from './settingsWidgets.js';

interface ConfigurationSettingsPaneOptions {
	readonly clipboardService: IClipboardService;
	readonly configurationService: IConfigurationService;
	readonly contextMenuProvider: IContextMenuProvider;
	readonly contextViewProvider: IContextViewProvider;
	readonly groups: readonly ConfigurationSettingsGroupDescriptor[];
	readonly note?: string;
	readonly presentation: SettingsWidgetsPresentation;
}

/** Composes Settings layout, tree projection, and per-item widgets for one pane. */
export class ConfigurationSettingsPane extends DisposableOwner {
	public readonly element: HTMLDivElement;
	private readonly status: HTMLParagraphElement;
	private readonly tree: SettingsTree<ConfigurationSettingDescriptor>;

	constructor(container: HTMLElement, options: ConfigurationSettingsPaneOptions) {
		super();
		const document = container.ownerDocument;
		this.element = h(document, 'div');
		this.element.className = `zeta-${options.presentation}-settings`;
		container.append(this.element);
		if (options.note) {
			const note = h(document, 'p');
			note.className = `zeta-${options.presentation}-settings-note`;
			note.textContent = options.note;
			this.element.append(note);
		}

		this.status = h(document, 'p');
		this.status.className = `zeta-${options.presentation}-settings-status`;
		this.status.setAttribute('role', 'status');
		this.status.setAttribute('aria-live', 'polite');

		const widgets = this.own(new SettingsWidgets(this.element, {
			clipboardService: options.clipboardService,
			configurationService: options.configurationService,
			contextMenuProvider: options.contextMenuProvider,
			contextViewProvider: options.contextViewProvider,
			onStatus: (message, isError) => this.showStatus(message, isError),
			presentation: options.presentation,
		}));
		const layout = new SettingsLayout(options.presentation, options.groups);
		const model = this.own(new SettingsTreeModel<ConfigurationSettingDescriptor>());
		model.setChildren(layout.nodes);
		this.tree = this.own(new SettingsTree(this.element, {
			model,
			rootClassName: `zeta-${options.presentation}-settings-tree`,
			groupClassName: `zeta-${options.presentation}-settings-group`,
			groupDescriptionClassName: `zeta-${options.presentation}-settings-group-description`,
			itemsClassName: `zeta-${options.presentation}-settings-list`,
			renderItem: item => widgets.render(item.value),
			disposeItem: item => widgets.disposeItem(item.id),
		}));
		this.element.append(this.status);
	}

	public setQuery(query: string): void {
		this.tree.setQuery(query);
	}

	private showStatus(message: string, isError: boolean): void {
		this.status.textContent = message;
		this.status.classList.toggle('is-error', isError);
	}
}
