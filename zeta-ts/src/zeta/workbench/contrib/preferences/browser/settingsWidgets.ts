import type { IContextMenuProvider } from '../../../../base/browser/contextmenu.js';
import { addDisposableListener, h } from '../../../../base/browser/dom.js';
import type { IContextViewProvider } from '../../../../base/browser/ui/contextview/contextview.js';
import { InputBox } from '../../../../base/browser/ui/inputbox/inputbox.js';
import { Button } from '../../../../base/browser/ui/button/button.js';
import { SelectBox } from '../../../../base/browser/ui/selectbox/selectbox.js';
import { Checkbox, Switch, type Toggle } from '../../../../base/browser/ui/toggle/toggle.js';
import { DisposableOwner, DisposableStore } from '../../../../base/common/lifecycle.js';
import type { IClipboardService } from '../../../../platform/clipboard/common/clipboardService.js';
import type { IConfigurationService } from '../../../../platform/configuration/common/configurationService.js';
import { configurationSettingBinding, SettingsItemModel, type SettingValueBinding, type SettingsItemState } from '../../../services/preferences/common/settingsModel.js';
import type {
	BooleanSettingDescriptor,
	ActionSettingDescriptor,
	BoundSelectSettingDescriptor,
	ConfigurationSettingDescriptor,
	InformationSettingDescriptor,
	NumberSettingDescriptor,
	SelectSettingDescriptor,
	TextSettingDescriptor,
} from '../common/settingsDescriptors.js';
import { SettingsItemActions } from './settingsItemActions.js';

export type SettingsWidgetsPresentation = 'general' | 'editor';

export interface SettingsWidgetsOptions {
	readonly clipboardService: IClipboardService;
	readonly configurationService: IConfigurationService;
	readonly contextMenuProvider: IContextMenuProvider;
	readonly contextViewProvider: IContextViewProvider;
	readonly onStatus: (message: string, isError: boolean) => void;
	readonly presentation: SettingsWidgetsPresentation;
}

interface RenderedSettingsWidget {
	readonly element: HTMLElement;
	readonly resources: DisposableStore;
}

/** Adapts typed Settings descriptors to controls without owning list topology. */
export class SettingsWidgets extends DisposableOwner {
	private readonly rendered = new Map<string, RenderedSettingsWidget>();
	private readonly document: Document;

	constructor(container: HTMLElement, private readonly options: SettingsWidgetsOptions) {
		super();
		this.document = container.ownerDocument;
		this.defer(() => {
			for (const widget of this.rendered.values()) widget.resources.dispose();
			this.rendered.clear();
		});
	}

	public render(descriptor: ConfigurationSettingDescriptor): HTMLElement {
		const existing = this.rendered.get(descriptor.id);
		if (existing) return existing.element;

		const resources = this.own(new DisposableStore());
		const element = this.renderDescriptor(descriptor, resources);
		this.rendered.set(descriptor.id, { element, resources });
		return element;
	}

	public disposeItem(id: string): void {
		const widget = this.rendered.get(id);
		if (!widget) return;
		widget.resources.dispose();
		this.rendered.delete(id);
	}

	private renderDescriptor(descriptor: ConfigurationSettingDescriptor, resources: DisposableStore): HTMLElement {
		switch (descriptor.kind) {
			case 'action':
				return this.renderActionSetting(descriptor, resources);
			case 'boolean':
				return this.renderBooleanSetting(descriptor, resources);
			case 'number':
				return this.renderNumberSetting(descriptor, resources);
			case 'select':
				return this.renderSelectSetting(descriptor, configurationSettingBinding(this.options.configurationService, descriptor.key), resources);
			case 'boundSelect':
				return this.renderSelectSetting(descriptor, descriptor.binding, resources);
			case 'text':
				return this.renderTextSetting(descriptor, resources);
			case 'information':
				return this.renderInformationSetting(descriptor);
		}
	}

	private renderActionSetting(descriptor: ActionSettingDescriptor, resources: DisposableStore): HTMLElement {
		const setting = this.createSettingDomNode();
		setting.append(this.createSettingCopy(descriptor.title, descriptor.description));
		const button = resources.add(new Button(setting, {
			label: descriptor.actionLabel,
			onClick: () => {
				button.enabled = false;
				Promise.resolve(descriptor.run()).catch(error => {
					this.options.onStatus(error instanceof Error ? error.message : 'Unable to run the setting action.', true);
				}).finally(() => {
					button.enabled = true;
				});
			},
		}));
		button.element.classList.add(`zeta-${this.options.presentation}-setting-action`);
		button.element.dataset.settingActionId = descriptor.id;
		return setting;
	}

	private renderBooleanSetting(descriptor: BooleanSettingDescriptor, resources: DisposableStore): HTMLElement {
		const setting = this.createSettingDomNode('toggle');
		const copy = this.createSettingCopy(descriptor.title, descriptor.description);
		const toggle: Toggle = this.options.presentation === 'general'
			? resources.add(new Checkbox(setting, { ariaLabel: descriptor.title, content: copy, contentPlacement: 'before-control' }))
			: resources.add(new Switch(setting, { ariaLabel: descriptor.title, content: copy, contentPlacement: 'before-control' }));
		toggle.element.classList.add(`zeta-${this.options.presentation}-toggle-control`);
		toggle.input.dataset.configurationKey = descriptor.key.key;
		const model = this.bindSetting(setting, descriptor, configurationSettingBinding(this.options.configurationService, descriptor.key), state => {
			toggle.checked = state.value;
			toggle.enabled = !state.isPending;
		}, resources);
		resources.add(toggle.onDidChange(checked => void this.updateSetting(model, checked)));
		return setting;
	}

	private renderNumberSetting(descriptor: NumberSettingDescriptor, resources: DisposableStore): HTMLElement {
		const setting = this.createSettingDomNode();
		setting.append(this.createSettingCopy(descriptor.title, descriptor.description));
		const binding = configurationSettingBinding(this.options.configurationService, descriptor.key);
		if (this.options.presentation === 'editor') {
			const input = resources.add(new InputBox(setting, {
				type: 'number',
				ariaLabel: descriptor.title,
				presentation: 'field',
			}));
			input.element.classList.add('zeta-editor-setting-number');
			input.inputElement.min = String(descriptor.minimum);
			input.inputElement.max = String(descriptor.maximum);
			input.step = '1';
			input.inputElement.dataset.configurationKey = descriptor.key.key;
			setting.append(input.element);
			const model = this.bindSetting(setting, descriptor, binding, state => {
				input.value = String(state.value);
				input.enabled = !state.isPending;
			}, resources);
			resources.add(addDisposableListener(input.inputElement, 'change', () => this.acceptNumberValue(model, input.inputElement, descriptor)));
			return setting;
		}

		const input = h(this.document, 'input');
		input.className = 'zeta-general-setting-control';
		input.type = 'number';
		input.min = String(descriptor.minimum);
		input.max = String(descriptor.maximum);
		input.step = '1';
		input.setAttribute('aria-label', descriptor.title);
		input.dataset.configurationKey = descriptor.key.key;
		setting.append(input);
		const model = this.bindSetting(setting, descriptor, binding, state => {
			input.value = String(state.value);
			input.disabled = state.isPending;
		}, resources);
		resources.add(addDisposableListener(input, 'change', () => this.acceptNumberValue(model, input, descriptor)));
		return setting;
	}

	private renderSelectSetting(descriptor: SelectSettingDescriptor | BoundSelectSettingDescriptor, binding: SettingValueBinding<string>, resources: DisposableStore): HTMLElement {
		const setting = this.createSettingDomNode('select');
		const select = resources.add(new SelectBox(setting, {
			options: descriptor.options,
			ariaLabel: descriptor.title,
			presentation: 'field',
			contextViewProvider: this.options.contextViewProvider,
		}));
		select.element.classList.add(this.options.presentation === 'general' ? 'zeta-general-setting-control' : 'zeta-editor-setting-select');
		select.element.dataset.configurationKey = descriptor.id;
		setting.append(this.createSettingCopy(descriptor.title, descriptor.description), select.element);
		const model = this.bindSetting(setting, descriptor, binding, state => {
			select.value = state.value;
			select.enabled = !state.isPending;
		}, resources);
		resources.add(select.onDidSelect(({ value }) => void this.updateSetting(model, value)));
		return setting;
	}

	private renderTextSetting(descriptor: TextSettingDescriptor, resources: DisposableStore): HTMLElement {
		const setting = this.createSettingDomNode();
		const input = h(this.document, 'input');
		input.className = `zeta-${this.options.presentation}-setting-text`;
		input.type = 'text';
		input.placeholder = descriptor.placeholder;
		input.setAttribute('aria-label', descriptor.title);
		input.dataset.configurationKey = descriptor.key.key;
		setting.append(this.createSettingCopy(descriptor.title, descriptor.description), input);
		const model = this.bindSetting(setting, descriptor, configurationSettingBinding(this.options.configurationService, descriptor.key), state => {
			input.value = state.value;
			input.disabled = state.isPending;
		}, resources);
		resources.add(addDisposableListener(input, 'change', () => void this.updateSetting(model, input.value.trim())));
		return setting;
	}

	private renderInformationSetting(descriptor: InformationSettingDescriptor): HTMLElement {
		const setting = this.createSettingDomNode('information');
		setting.append(this.createSettingCopy(descriptor.title, descriptor.description));
		const state = h(this.document, 'span');
		state.className = `zeta-${this.options.presentation}-setting-state`;
		state.textContent = descriptor.stateLabel;
		setting.append(state);
		return setting;
	}

	private bindSetting<T>(setting: HTMLElement, descriptor: ConfigurationSettingDescriptor, binding: SettingValueBinding<T>, renderState: (state: SettingsItemState<T>) => void, resources: DisposableStore): SettingsItemModel<T> {
		const model = resources.add(new SettingsItemModel(binding));
		resources.add(model.onDidChange(renderState));
		renderState(model.state);
		resources.add(new SettingsItemActions(setting, {
			label: descriptor.title,
			reference: {
				id: model.id,
				isDefault: () => model.isDefault(),
				reset: () => this.resetSetting(model),
			},
			contextMenuProvider: this.options.contextMenuProvider,
			clipboardService: this.options.clipboardService,
			onError: error => this.options.onStatus(error instanceof Error ? error.message : 'Unable to run the setting action.', true),
		}));
		return model;
	}

	private createSettingDomNode(kind?: 'information' | 'select' | 'toggle'): HTMLDivElement {
		const setting = h(this.document, 'div');
		setting.className = `zeta-${this.options.presentation}-setting`;
		if (kind === 'information') setting.classList.add(`zeta-${this.options.presentation}-informational-setting`);
		if (kind === 'select' && this.options.presentation === 'editor') setting.classList.add('zeta-editor-setting-select-row');
		if (kind === 'toggle') setting.classList.add(`zeta-${this.options.presentation}-toggle-setting`);
		return setting;
	}

	private createSettingCopy(label: string, description: string): HTMLElement {
		const copy = h(this.document, 'span');
		copy.className = `zeta-${this.options.presentation}-setting-copy`;
		const title = h(this.document, 'span');
		title.className = `zeta-${this.options.presentation}-setting-title`;
		title.textContent = label;
		const hint = h(this.document, 'span');
		hint.className = `zeta-${this.options.presentation}-setting-description`;
		hint.textContent = description;
		copy.append(title, hint);
		return copy;
	}

	private acceptNumberValue(model: SettingsItemModel<number>, input: HTMLInputElement, descriptor: NumberSettingDescriptor): void {
		const value = input.valueAsNumber;
		if (!Number.isSafeInteger(value) || value < descriptor.minimum || value > descriptor.maximum) {
			model.refresh();
			input.value = String(model.state.value);
			this.options.onStatus(`${descriptor.title} must be between ${descriptor.minimum} and ${descriptor.maximum}.`, true);
			return;
		}
		void this.updateSetting(model, value);
	}

	private async updateSetting<T>(model: SettingsItemModel<T>, value: T): Promise<void> {
		try {
			await model.update(value);
			this.options.onStatus('Setting saved.', false);
		} catch (error) {
			this.options.onStatus(error instanceof Error ? error.message : 'Unable to save the setting.', true);
		}
	}

	private async resetSetting<T>(model: SettingsItemModel<T>): Promise<void> {
		try {
			await model.reset();
			this.options.onStatus('Setting reset.', false);
		} catch (error) {
			this.options.onStatus(error instanceof Error ? error.message : 'Unable to reset the setting.', true);
			throw error;
		}
	}
}
