import './media/settingsRenderers.css';
import type { IContextMenuProvider } from '../../../../base/browser/contextmenu.js';
import { addDisposableListener, h } from '../../../../base/browser/dom.js';
import { Button } from '../../../../base/browser/ui/button/button.js';
import type { IContextViewProvider } from '../../../../base/browser/ui/contextview/contextview.js';
import { InputBox } from '../../../../base/browser/ui/inputbox/inputbox.js';
import { SelectBox } from '../../../../base/browser/ui/selectbox/selectbox.js';
import { Checkbox, Switch, type Toggle } from '../../../../base/browser/ui/toggle/toggle.js';
import type { IAction } from '../../../../base/common/actions.js';
import { DisposableOwner, DisposableStore } from '../../../../base/common/lifecycle.js';
import { lxiconsLibrary } from '../../../../base/common/lxiconsLibrary.js';
import type { IClipboardService } from '../../../../platform/clipboard/common/clipboardService.js';
import type { IConfigurationService } from '../../../../platform/configuration/common/configurationService.js';
import type {
	IBooleanSetting,
	INumberSetting,
	ISelectSetting,
	ISetting,
	ITextSetting,
	SettingReference,
	SettingValueBinding,
	SettingsPresentation,
} from '../../../services/preferences/common/preferences.js';
import { configurationSettingBinding, SettingModel, type SettingState } from '../../../services/preferences/common/preferencesModels.js';

export interface SettingsRenderersOptions {
	readonly clipboardService: IClipboardService;
	readonly configurationService: IConfigurationService;
	readonly contextMenuProvider: IContextMenuProvider;
	readonly contextViewProvider: IContextViewProvider;
	readonly onStatus: (message: string, isError: boolean) => void;
}

interface SettingActionsOptions {
	readonly label: string;
	readonly reference: SettingReference;
	readonly contextMenuProvider: IContextMenuProvider;
	readonly clipboardService: IClipboardService;
	readonly onError?: (error: unknown) => void;
}

/** Gear menu attached to one Configuration Registry-backed setting row. */
class SettingActions extends DisposableOwner {
	private readonly element: HTMLSpanElement;
	private readonly trigger: Button;

	constructor(container: HTMLElement, private readonly options: SettingActionsOptions) {
		super();
		this.element = h(container.ownerDocument, 'span');
		this.element.className = 'zeta-setting-item-actions';
		this.trigger = this.own(new Button(this.element, {
			label: `More actions for ${options.label}`,
			title: `More actions for ${options.label}`,
			icon: lxiconsLibrary.gear,
			onClick: () => this.show(),
		}));
		this.trigger.toggleClassName('zeta-setting-item-actions-trigger', true);
		this.trigger.domNode.setAttribute('aria-label', `More actions for ${options.label}`);
		this.trigger.domNode.setAttribute('aria-haspopup', 'menu');
		this.trigger.domNode.setAttribute('aria-expanded', 'false');
		container.dataset.settingsItemId = options.reference.id;
		container.dataset.settingsItemKind = 'setting';
		container.classList.add('zeta-setting-item');
		container.prepend(this.element);
		this.defer(() => {
			container.classList.remove('zeta-setting-item');
			this.element.remove();
		});
	}

	private show(): void {
		if (this.element.classList.contains('is-open')) return;
		const actions: readonly IAction[] = [
			{
				id: 'settings.resetSetting',
				label: 'Reset Setting',
				tooltip: '',
				enabled: !this.options.reference.isDefault(),
				run: () => this.run(this.options.reference.reset),
			},
			{
				id: 'settings.copySettingId',
				label: 'Copy Setting ID',
				tooltip: '',
				enabled: true,
				run: () => this.run(() => this.options.clipboardService.writeText(this.options.reference.id)),
			},
		];
		this.setOpen(true);
		try {
			this.options.contextMenuProvider.showContextMenu({
				anchor: this.trigger.domNode,
				actions,
				onHide: () => this.setOpen(false),
			});
		} catch (error) {
			this.setOpen(false);
			this.options.onError?.(error);
		}
	}

	private run(operation: () => Promise<void>): void {
		void operation().catch(error => this.options.onError?.(error));
	}

	private setOpen(open: boolean): void {
		this.element.classList.toggle('is-open', open);
		this.trigger.domNode.setAttribute('aria-expanded', String(open));
	}
}

interface RenderedSetting {
	readonly element: HTMLElement;
	readonly resources: DisposableStore;
}

/** Owns the generic controls for Configuration Registry settings. */
export class SettingsRenderers extends DisposableOwner {
	private readonly rendered = new Map<string, RenderedSetting>();
	private readonly document: Document;

	constructor(container: HTMLElement, private readonly options: SettingsRenderersOptions) {
		super();
		this.document = container.ownerDocument;
		this.defer(() => {
			for (const widget of this.rendered.values()) {
				widget.resources.dispose();
			}
			this.rendered.clear();
		});
	}

	public render(setting: ISetting): HTMLElement {
		const existing = this.rendered.get(setting.id);
		if (existing) return existing.element;

		const resources = this.own(new DisposableStore());
		const element = this.renderSetting(setting, resources);
		this.rendered.set(setting.id, { element, resources });
		return element;
	}

	public update(setting: ISetting): void {
		const rendered = this.rendered.get(setting.id);
		if (!rendered) return;
		rendered.element.querySelector<HTMLElement>('.zeta-configuration-setting-title')!.textContent = setting.title;
		rendered.element.querySelector<HTMLElement>('.zeta-configuration-setting-description')!.textContent = setting.description;
	}

	public disposeSetting(id: string): void {
		const rendered = this.rendered.get(id);
		if (!rendered) return;
		rendered.resources.dispose();
		this.rendered.delete(id);
	}

	private renderSetting(setting: ISetting, resources: DisposableStore): HTMLElement {
		switch (setting.valueType) {
			case 'boolean':
				return this.renderBooleanSetting(setting, resources);
			case 'number':
				return this.renderNumberSetting(setting, resources);
			case 'select':
				return this.renderSelectSetting(setting, configurationSettingBinding(this.options.configurationService, setting.key), resources);
			case 'text':
				return this.renderTextSetting(setting, resources);
		}
	}

	private renderBooleanSetting(descriptor: IBooleanSetting, resources: DisposableStore): HTMLElement {
		const presentation = this.presentation(descriptor);
		const setting = this.createSettingDomNode(presentation, 'toggle');
		const copy = this.createSettingCopy(descriptor);
		const toggle: Toggle = presentation === 'general'
			? resources.add(new Checkbox(setting, { ariaLabel: descriptor.title, content: copy, contentPlacement: 'before-control' }))
			: resources.add(new Switch(setting, { ariaLabel: descriptor.title, content: copy, contentPlacement: 'before-control' }));
		toggle.element.classList.add(`zeta-${presentation}-toggle-control`);
		toggle.input.dataset.configurationKey = descriptor.key.key;
		const model = this.bindSetting(setting, descriptor, configurationSettingBinding(this.options.configurationService, descriptor.key), state => {
			toggle.checked = state.value;
			toggle.busy = state.isPending;
		}, resources);
		resources.add(toggle.onDidChange(checked => void this.updateSetting(model, checked)));
		return setting;
	}

	private renderNumberSetting(descriptor: INumberSetting, resources: DisposableStore): HTMLElement {
		const presentation = this.presentation(descriptor);
		const setting = this.createSettingDomNode(presentation);
		setting.append(this.createSettingCopy(descriptor));
		const binding = configurationSettingBinding(this.options.configurationService, descriptor.key);
		if (presentation === 'editor') {
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

	private renderSelectSetting(descriptor: ISelectSetting, binding: SettingValueBinding<string>, resources: DisposableStore): HTMLElement {
		const presentation = this.presentation(descriptor);
		const setting = this.createSettingDomNode(presentation, 'select');
		const select = resources.add(new SelectBox(setting, {
			options: descriptor.options,
			ariaLabel: descriptor.title,
			presentation: 'field',
			contextViewProvider: this.options.contextViewProvider,
		}));
		select.element.classList.add(presentation === 'general' ? 'zeta-general-setting-control' : 'zeta-editor-setting-select');
		select.element.dataset.configurationKey = descriptor.id;
		setting.append(this.createSettingCopy(descriptor), select.element);
		const model = this.bindSetting(setting, descriptor, binding, state => {
			select.value = state.value;
			select.enabled = !state.isPending;
		}, resources);
		resources.add(select.onDidSelect(({ value }) => void this.updateSetting(model, value)));
		return setting;
	}

	private renderTextSetting(descriptor: ITextSetting, resources: DisposableStore): HTMLElement {
		const presentation = this.presentation(descriptor);
		const setting = this.createSettingDomNode(presentation);
		const input = h(this.document, 'input');
		input.className = `zeta-${presentation}-setting-text`;
		input.type = 'text';
		input.placeholder = descriptor.placeholder;
		input.setAttribute('aria-label', descriptor.title);
		input.dataset.configurationKey = descriptor.key.key;
		setting.append(this.createSettingCopy(descriptor), input);
		const model = this.bindSetting(setting, descriptor, configurationSettingBinding(this.options.configurationService, descriptor.key), state => {
			input.value = state.value;
			input.disabled = state.isPending;
		}, resources);
		resources.add(addDisposableListener(input, 'change', () => void this.updateSetting(model, input.value.trim())));
		return setting;
	}

	private bindSetting<T>(setting: HTMLElement, descriptor: ISetting, binding: SettingValueBinding<T>, renderState: (state: SettingState<T>) => void, resources: DisposableStore): SettingModel<T> {
		const model = resources.add(new SettingModel(binding));
		resources.add(model.onDidChange(renderState));
		renderState(model.state);
		resources.add(new SettingActions(setting, {
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

	private createSettingDomNode(presentation: SettingsPresentation, kind?: 'select' | 'toggle'): HTMLDivElement {
		const setting = h(this.document, 'div');
		setting.className = `zeta-configuration-setting zeta-${presentation}-setting`;
		if (kind === 'select' && presentation === 'editor') setting.classList.add('zeta-editor-setting-select-row');
		if (kind === 'toggle') setting.classList.add(`zeta-${presentation}-toggle-setting`);
		return setting;
	}

	private createSettingCopy(setting: ISetting): HTMLElement {
		const presentation = this.presentation(setting);
		const copy = h(this.document, 'span');
		copy.className = `zeta-configuration-setting-copy zeta-${presentation}-setting-copy`;
		const title = h(this.document, 'span');
		title.className = `zeta-configuration-setting-title zeta-${presentation}-setting-title`;
		title.textContent = setting.title;
		const hint = h(this.document, 'span');
		hint.className = `zeta-configuration-setting-description zeta-${presentation}-setting-description`;
		hint.textContent = setting.description;
		copy.append(title, hint);
		return copy;
	}

	private acceptNumberValue(model: SettingModel<number>, input: HTMLInputElement, descriptor: INumberSetting): void {
		const value = input.valueAsNumber;
		if (!Number.isSafeInteger(value) || value < descriptor.minimum || value > descriptor.maximum) {
			model.refresh();
			input.value = String(model.state.value);
			this.options.onStatus(`${descriptor.title} must be between ${descriptor.minimum} and ${descriptor.maximum}.`, true);
			return;
		}
		void this.updateSetting(model, value);
	}

	private async updateSetting<T>(model: SettingModel<T>, value: T): Promise<void> {
		this.options.onStatus('', false);
		try {
			await model.update(value);
		} catch (error) {
			this.options.onStatus(error instanceof Error ? error.message : 'Unable to save the setting.', true);
		}
	}

	private async resetSetting<T>(model: SettingModel<T>): Promise<void> {
		this.options.onStatus('', false);
		try {
			await model.reset();
		} catch (error) {
			this.options.onStatus(error instanceof Error ? error.message : 'Unable to reset the setting.', true);
			throw error;
		}
	}

	private presentation(setting: ISetting): SettingsPresentation {
		return setting.presentation ?? 'editor';
	}
}
